//! The kernel's rates, measured rather than tested: cell-updates per second whole, a step at a
//! time by tiles and several steps at a time by diamonds, on cubes from the caches' size to far
//! beyond them, against the memory's roof. Run alone on the machine:
//!
//! ```sh
//! cargo test --release --lib fdtd::kernel_rates -- --ignored --nocapture --test-threads 1
//! ```
//!
//! `kernel_scan` times chosen tiles (`FDTD_SIZES=128,320 FDTD_BLOCKINGS=0:0,16:1,10:4d
//! FDTD_THREADS=1,20 FDTD_PRECISION=f64,f32`, `rows:steps`, `d` for diamonds, `0:0` whole).

use std::time::Instant;

use super::checks::{grid, noise};
use super::kernel::{Blocking, Tiling};
use super::*;

/// A cube of `n` cells of 50 nm of vacuum inside CPMLs of 8, random fields.
fn filled(n: usize) -> Simulation {
    let mut s =
        Simulation::in_uniform_medium(grid([n; 3], 0.05), 1.0, Boundaries::cpml(8), 0.9).unwrap();
    for c in Axis::ALL {
        let k = c.index();
        for (r, v) in s.e_mut(c).iter_mut().enumerate() {
            *v = noise(r + 3 * k);
        }
        for (r, v) in s.h_mut(c).iter_mut().enumerate() {
            *v = noise(r + 7 * k + 50_000);
        }
    }
    s
}

/// The best of three runs of `yee` by `blocking`, in cell-updates per second, each run of
/// enough steps to take a few tenths of a second.
fn rate<T: Real>(yee: &mut Yee<T>, blocking: Option<Blocking>) -> f64 {
    let cells = yee.cells() as f64;
    let block = blocking.map_or(1, |b| 2 * b.steps);
    let mut steps = block.max(4);
    yee.run(steps, blocking);
    loop {
        let t = Instant::now();
        yee.run(steps, blocking);
        if t.elapsed().as_secs_f64() > 0.3 {
            break;
        }
        steps *= 2;
    }
    (0..3)
        .map(|_| {
            let t = Instant::now();
            yee.run(steps, blocking);
            cells * steps as f64 / t.elapsed().as_secs_f64()
        })
        .fold(0.0, f64::max)
}

fn pool(threads: usize) -> rayon::ThreadPool {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .unwrap()
}

#[test]
#[ignore = "a measurement: run alone, in release"]
fn kernel_rates() {
    let threads = [1, 20];
    let triad: Vec<f64> = threads
        .iter()
        .map(|&t| pool(t).install(|| crate::bench::triad(1 << 24, 10)))
        .collect();
    println!("triad: {triad:?} GB/s on {threads:?} threads");
    println!(
        "| Grid | Precision | Threads | Whole | A step at a time by tiles | Diamonds | The roof |"
    );
    println!("|---|---|---:|---:|---:|---:|---:|");
    fn rows<T: Real>(n: usize, threads: &[usize], triad: &[f64]) {
        let s = filled(n);
        let mut yee = Yee::<T>::from_simulation(&s).unwrap();
        drop(s);
        let g = grid([n; 3], 0.05);
        let size = std::mem::size_of::<T>();
        let per_update = yee.bytes_per_step() as f64 / yee.cells() as f64;
        for (&t, &gb) in threads.iter().zip(triad) {
            pool(t).install(|| {
                let spatial = Blocking::auto(g, size, false, t);
                let diamonds = Blocking::auto(g, size, true, t);
                let whole = rate(&mut yee, None);
                let one = spatial.map(|b| rate(&mut yee, Some(b)));
                let many = diamonds.map(|b| (b, rate(&mut yee, Some(b))));
                let m = |r: f64| format!("{:.0}", r / 1e6);
                println!(
                    "| {n}³ | {} | {t} | {} | {} | {} | {:.0} |",
                    std::any::type_name::<T>(),
                    m(whole),
                    one.map_or("—".into(), m),
                    many.map_or("—".into(), |(b, r)| format!(
                        "{} ({} rows, {} steps)",
                        m(r),
                        b.rows,
                        b.steps
                    )),
                    gb * 1e9 / per_update / 1e6
                );
            });
        }
    }
    for n in [64, 96, 128, 192, 256, 320] {
        rows::<f64>(n, &threads, &triad);
        rows::<f32>(n, &threads, &triad);
    }
}

#[test]
#[ignore = "a measurement: run alone, in release"]
fn kernel_scan() {
    let list = |name: &str, default: &str| -> Vec<String> {
        std::env::var(name)
            .unwrap_or(default.into())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };
    let sizes: Vec<usize> = list("FDTD_SIZES", "128,320")
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let blockings: Vec<Option<Blocking>> = list("FDTD_BLOCKINGS", "0:0,16:1,10:4d")
        .iter()
        .map(|s| {
            let (r, st) = s.split_once(':').unwrap();
            let diamonds = st.ends_with('d');
            let (rows, steps) = (
                r.parse().unwrap(),
                st.trim_end_matches('d').parse().unwrap(),
            );
            (steps > 0).then_some(Blocking {
                rows,
                steps,
                diamonds,
            })
        })
        .collect();
    let threads: Vec<usize> = list("FDTD_THREADS", "1,20")
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
    let precisions = list("FDTD_PRECISION", "f64,f32");
    fn scan<T: Real>(n: usize, blockings: &[Option<Blocking>], threads: &[usize]) {
        let s = filled(n);
        let mut yee = Yee::<T>::from_simulation(&s).unwrap();
        drop(s);
        for &t in threads {
            for &b in blockings {
                let r = pool(t).install(|| rate(&mut yee, b));
                println!(
                    "{n}³ {} {t:>2} threads {:>14}: {:7.1} M cell-updates/s",
                    std::any::type_name::<T>(),
                    b.map_or("whole".into(), |b| format!(
                        "{}r×{}s{}",
                        b.rows,
                        b.steps,
                        if b.diamonds { " diamonds" } else { "" }
                    )),
                    r / 1e6
                );
            }
        }
    }
    for &n in &sizes {
        for p in &precisions {
            match p.as_str() {
                "f64" => scan::<f64>(n, &blockings, &threads),
                _ => scan::<f32>(n, &blockings, &threads),
            }
        }
    }
}

/// A simulation's rate with a source, a probe and a flux box (six transformed faces at three
/// frequencies): whole, a step at a time by tiles, and run by diamonds.
#[test]
#[ignore = "a measurement: run alone, in release"]
fn simulation_rates() {
    for n in [128, 256] {
        let mut s = filled(n);
        s.add_source(Source {
            field: Field::E,
            component: Axis::Z,
            at: (n / 2, n / 2, n / 2),
            waveform: Waveform::pulse(Frequency::natural(1.0).unwrap(), 0.6).unwrap(),
        })
        .unwrap();
        s.add_probe(Field::E, Axis::Z, (n / 3, n / 2, n / 2))
            .unwrap();
        let frequencies = [0.8, 1.0, 1.2].map(|f| Frequency::natural(f).unwrap());
        let (a, b) = (n / 4, 3 * n / 4);
        s.add_flux_box([(a, b), (a, b), (a, b)], &frequencies)
            .unwrap();
        for threads in [1, 20] {
            for (what, tiling, stepped) in [
                ("whole", Tiling::Whole, true),
                ("a step at a time by tiles", Tiling::Auto, true),
                ("run by diamonds", Tiling::Auto, false),
            ] {
                let mut x = s.clone();
                x.set_tiling(tiling);
                let steps = 40;
                let rate = pool(threads).install(|| {
                    let t = Instant::now();
                    if stepped {
                        (0..steps).for_each(|_| x.step());
                    } else {
                        x.run(steps);
                    }
                    (n * n * n * steps) as f64 / t.elapsed().as_secs_f64()
                });
                println!(
                    "{n}³ {threads:>2} threads, {what}: {:.0} M cell-updates/s",
                    rate / 1e6
                );
            }
        }
    }
}

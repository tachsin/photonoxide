use super::*;

#[test]
#[ignore = "a measurement: SYMMETRIC=1 (the symmetric forms), AMD=1 (AMD, not nested dissection), RAYON_NUM_THREADS; cargo test --release sparse::multifrontal::measure -- --ignored --nocapture"]
fn on_the_exported_systems() {
    for id in ["slab-2d", "strip-24", "strip-32"] {
        let system = crate::bench::export::system(id).unwrap();
        // SYMMETRIC=1: its complex symmetric form, B = S A S⁻¹, by L D Lᵀ
        let symmetric = std::env::var("SYMMETRIC").is_ok();
        let system = if symmetric {
            let zero = vec![c64::new(0.0, 0.0); system.n];
            crate::bench::export::symmetric(&system, &zero).unwrap().0
        } else {
            system
        };
        let n = system.n;
        let t: Vec<Triplet<usize, usize, c64>> = system
            .entries
            .iter()
            .map(|&(r, c, v)| Triplet::new(r, c, v))
            .collect();
        let a = SparseColMat::<usize, c64>::try_new_from_triplets(n, n, &t).unwrap();
        let positions = if std::env::var("AMD").is_ok() {
            None
        } else {
            crate::bench::export::positions(&system)
        };
        let t0 = std::time::Instant::now();
        let analysis = Arc::new(if symmetric {
            Analysis::new_symmetric(a.as_ref(), positions.as_deref())
                .unwrap()
                .unwrap()
        } else {
            Analysis::new(a.as_ref(), positions.as_deref()).unwrap()
        });
        let t1 = std::time::Instant::now();
        let lu = Multifrontal::new(analysis.clone(), a.as_ref()).unwrap();
        let t2 = std::time::Instant::now();
        let x = lu.solve(&system.rhs);
        let t3 = std::time::Instant::now();
        let mut r = system.rhs.clone();
        for &(i, j, v) in &system.entries {
            r[i] -= v * x[j];
        }
        let norm = |v: &[c64]| v.iter().map(|z| z.norm_sqr()).sum::<f64>().sqrt();
        let flops: f64 = analysis
            .fronts
            .iter()
            .map(|f| {
                let k = (f.end - f.begin) as f64;
                let p = f.below.len() as f64;
                8.0 * (k * k * k / 3.0 + k * k * p + k * p * p)
            })
            .sum();
        eprintln!(
            "{id}: {:.1} Gflop, {:.1} Gflop/s",
            flops / 1e9,
            flops / 1e9 / (t2 - t1).as_secs_f64()
        );
        let identity = analysis
            .matching
            .rows
            .iter()
            .enumerate()
            .all(|(j, &r)| j == r);
        eprintln!(
            "{id}: analysis {:.3} s, factorization {:.3} s, solve {:.3} s, entries {:.1} M, fronts {}, largest {}, perturbed {}, matching identity {identity}, residual {:.1e}",
            (t1 - t0).as_secs_f64(),
            (t2 - t1).as_secs_f64(),
            (t3 - t2).as_secs_f64(),
            analysis.factor_entries() as f64 / 1e6,
            analysis.fronts.len(),
            analysis
                .fronts
                .iter()
                .map(|f| f.end - f.begin + f.below.len())
                .max()
                .unwrap_or(0),
            lu.perturbed(),
            norm(&r) / norm(&system.rhs)
        );
    }
}

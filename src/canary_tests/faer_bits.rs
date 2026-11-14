//! What faer's dense kernels do on this system, bit for bit (#822): a 64-bit hash of the exact
//! bits of each result, on inputs whose bits are the same everywhere (53 random bits from an
//! integer generator, scaled to [−4, 4)), each computed with faer sequential and on rayon pools of
//! 1 to 5 threads. Shared by `src/canary_tests.rs` (the least squares, which the public API
//! doesn't reach) and `tests/canary.rs` (faer's own calls), which takes this file by `#[path]`.
//!
//! faer picks its kernels at run time: the product through nano-gemm when M·N·K ≤ 4096, else
//! private-gemm-x86 (AVX-512 when the CPU has avx512f, else AVX2 and FMA) on x86-64 and gemm
//! (NEON) on ARM; the LU, QR, eigenvalue and SVD code through pulp's `Arch::new()`. Nothing
//! switches that off, so the hashes say which path this system took. Every line goes to stderr
//! as `FAER <name> <size> <hash>`, then how the other ways compare; one `FAER-SYSTEM` line gives
//! the CPU's features and the crates' versions. The hashes are compared between systems by
//! hand, never asserted: whether they must agree is #822's open question.

#![allow(
    dead_code,
    reason = "the library's canary takes the real matrices only; tests/canary.rs takes the rest"
)]

use std::process::Command;

use faer::{ColRef, MatRef};
use num_complex::Complex64 as c64;

/// The variable that makes the test run in a child process with faer sequential.
const SEQUENTIAL: &str = "PHOTONOXIDE_FAER_CANARY_SEQUENTIAL";

/// One result: what was computed, on which size, and the hash of its bits.
#[derive(Clone, Debug)]
pub(crate) struct Item {
    pub(crate) name: String,
    pub(crate) size: String,
    pub(crate) hash: u64,
}

/// A value whose exact bits go into the hash.
pub(crate) trait Hashed: Copy {
    fn into_bits(self, h: &mut Bits);
}

impl Hashed for f64 {
    fn into_bits(self, h: &mut Bits) {
        h.word(self.to_bits());
    }
}

impl Hashed for c64 {
    fn into_bits(self, h: &mut Bits) {
        h.word(self.re.to_bits());
        h.word(self.im.to_bits());
    }
}

/// FNV-1a over the bytes of each value's bits, in order: a −0 is not a +0, and any reordering of
/// a sum that moves a last bit changes the hash.
pub(crate) struct Bits(u64);

impl Bits {
    pub(crate) fn new() -> Bits {
        Bits(0xcbf2_9ce4_8422_2325)
    }

    fn word(&mut self, w: u64) {
        for b in w.to_le_bytes() {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    pub(crate) fn values<'a, T: Hashed + 'a>(
        &mut self,
        vs: impl IntoIterator<Item = &'a T>,
    ) -> &mut Bits {
        for &v in vs {
            v.into_bits(self);
        }
        self
    }

    /// A matrix, column by column (its shape too).
    pub(crate) fn mat<T: Hashed>(&mut self, m: MatRef<'_, T>) -> &mut Bits {
        self.word(m.nrows() as u64);
        self.word(m.ncols() as u64);
        for j in 0..m.ncols() {
            for i in 0..m.nrows() {
                m[(i, j)].into_bits(self);
            }
        }
        self
    }

    pub(crate) fn col<T: Hashed>(&mut self, c: ColRef<'_, T>) -> &mut Bits {
        self.word(c.nrows() as u64);
        for i in 0..c.nrows() {
            c[i].into_bits(self);
        }
        self
    }

    pub(crate) fn finish(&self) -> u64 {
        self.0
    }
}

/// Inputs with the same bits on every system: 53 random bits from a linear congruential
/// generator seeded by `seed`, a double in [0, 1) exactly, then − ½ and × 8 (IEEE roundings).
/// Full mantissas, so that the order of a sum shows in its bits: with short ones (multiples of
/// 1/64) every product is exact and every sum of them exact in any order, and the products came
/// out the same bits on x86-64 and ARM whatever the kernel.
pub(crate) struct Inputs(u64);

impl Inputs {
    pub(crate) fn new(seed: u64) -> Inputs {
        Inputs(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ 0x2026_1114_FAE2_0822)
    }

    pub(crate) fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64 / 9_007_199_254_740_992.0 - 0.5) * 8.0
    }
}

/// An m × n real matrix of [`Inputs`].
pub(crate) fn real(m: usize, n: usize, seed: u64) -> faer::Mat<f64> {
    let mut g = Inputs::new(seed);
    faer::Mat::from_fn(m, n, |_, _| g.next())
}

/// An m × n complex matrix of [`Inputs`].
pub(crate) fn complex(m: usize, n: usize, seed: u64) -> faer::Mat<c64> {
    let mut g = Inputs::new(seed);
    faer::Mat::from_fn(m, n, |_, _| {
        let re = g.next();
        c64::new(re, g.next())
    })
}

fn on_threads<R: Send>(threads: usize, f: impl FnOnce() -> R + Send) -> R {
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("a thread pool")
        .install(f)
}

/// The CPU's features faer's kernels choose by, its name where the system gives it, and the
/// versions of faer and its kernels' crates (from Cargo.lock), as one line.
pub(crate) fn system() -> String {
    let mut s = format!("FAER-SYSTEM arch={}", std::env::consts::ARCH);
    #[cfg(target_arch = "x86_64")]
    {
        let flags = [
            ("avx", std::arch::is_x86_feature_detected!("avx")),
            ("avx2", std::arch::is_x86_feature_detected!("avx2")),
            ("fma", std::arch::is_x86_feature_detected!("fma")),
            ("avx512f", std::arch::is_x86_feature_detected!("avx512f")),
        ];
        for (f, on) in flags {
            s.push_str(&format!(" {f}={on}"));
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        let flags = [
            ("neon", std::arch::is_aarch64_feature_detected!("neon")),
            ("sve", std::arch::is_aarch64_feature_detected!("sve")),
        ];
        for (f, on) in flags {
            s.push_str(&format!(" {f}={on}"));
        }
    }
    s.push_str(&format!(
        " os={} cores={} rayon-threads={}",
        std::env::consts::OS,
        std::thread::available_parallelism().map_or(0, usize::from),
        rayon::current_num_threads()
    ));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap_or_default();
    // Cargo.lock's lines may end in CRLF (a Windows checkout)
    let lines: Vec<&str> = lock.lines().map(str::trim_end).collect();
    for name in [
        "faer",
        "pulp",
        "gemm",
        "nano-gemm",
        "private-gemm-x86",
        "rayon",
    ] {
        let at = format!("name = \"{name}\"");
        let version = lines
            .iter()
            .position(|l| *l == at)
            .and_then(|k| lines.get(k + 1))
            .and_then(|l| l.strip_prefix("version = "))
            .unwrap_or("?")
            .trim_matches('"');
        s.push_str(&format!(" {name}={version}"));
    }
    s.push_str(&format!(" cpu=\"{}\"", cpu_name()));
    s
}

fn cpu_name() -> String {
    let name = if cfg!(target_os = "linux") {
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|t| {
                // x86's model name; ARM's has none, but its implementer and part numbers
                let field = |key: &str| {
                    t.lines()
                        .find(|l| l.starts_with(key))
                        .and_then(|l| l.split(':').nth(1))
                        .map(|v| v.trim().to_owned())
                };
                field("model name").or_else(|| {
                    Some(format!(
                        "implementer {} part {}",
                        field("CPU implementer")?,
                        field("CPU part")?
                    ))
                })
            })
            .unwrap_or_default()
    } else if cfg!(target_os = "macos") {
        Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default()
    } else {
        std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_default()
    };
    name.trim().replace('"', "'")
}

/// The rayon pools every item is computed on: faer's default, `Par::rayon(0)`, takes the pool's
/// size, as the library's products and LUs do; its QR, SVDs and eigendecompositions run
/// sequentially whatever the pool (`src/dense/fixed.rs`).
const THREADS: [usize; 5] = [1, 2, 3, 4, 5];

/// What `compute` gives every way: faer sequential (in a child process, since faer's
/// parallelism is one global setting and the other tests run beside this one), on rayon pools of
/// 1 to 5 threads ([`THREADS`]), and on 5 threads again.
pub(crate) struct Ways {
    pub(crate) sequential: Vec<Item>,
    pub(crate) threads: Vec<(usize, Vec<Item>)>,
    pub(crate) again: Vec<Item>,
}

impl Ways {
    /// The items whose bits on some number of threads aren't the sequential ones.
    pub(crate) fn thread_dependent(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (k, s) in self.sequential.iter().enumerate() {
            if self.threads.iter().any(|(_, t)| t[k].hash != s.hash) {
                out.push(format!("{} {}", s.name, s.size));
            }
        }
        out
    }

    /// The items whose bits on 5 threads changed when computed again.
    pub(crate) fn unrepeatable(&self) -> Vec<String> {
        let five = &self.threads.last().expect("5 threads").1;
        five.iter()
            .zip(&self.again)
            .filter(|(a, b)| a.hash != b.hash)
            .map(|(a, _)| format!("{} {}", a.name, a.size))
            .collect()
    }
}

/// Runs `compute` every way ([`Ways`]) and prints each item's line. `test` is the test's full
/// name, for the child process; in the child (the variable set) it computes sequentially, prints
/// the lines its parent reads and returns `None`.
pub(crate) fn every_way(test: &str, compute: fn() -> Vec<Item>) -> Option<Ways> {
    if std::env::var_os(SEQUENTIAL).is_some() {
        faer::set_global_parallelism(faer::Par::Seq);
        for it in compute() {
            println!("FAER-SEQ {} {} {:016x}", it.name, it.size, it.hash);
        }
        return None;
    }
    let child = Command::new(std::env::current_exe().expect("the test binary"))
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(SEQUENTIAL, "1")
        .output()
        .expect("the test binary, again");
    let out = String::from_utf8_lossy(&child.stdout);
    assert!(
        child.status.success(),
        "the sequential run failed:\n{out}\n{}",
        String::from_utf8_lossy(&child.stderr)
    );
    let sequential: Vec<Item> = out
        .lines()
        // libtest's "test … " may begin the first line
        .filter_map(|l| l.find("FAER-SEQ ").map(|at| &l[at + "FAER-SEQ ".len()..]))
        .map(|l| {
            let w: Vec<&str> = l.split_whitespace().collect();
            Item {
                name: w[0].to_owned(),
                size: w[1].to_owned(),
                hash: u64::from_str_radix(w[2], 16).expect("a hash"),
            }
        })
        .collect();
    let threads: Vec<(usize, Vec<Item>)> = THREADS
        .into_iter()
        .map(|t| (t, on_threads(t, compute)))
        .collect();
    let again = on_threads(5, compute);
    let ways = Ways {
        sequential,
        threads,
        again,
    };
    assert_eq!(
        ways.sequential.len(),
        ways.again.len(),
        "the sequential run gave {} items, the threaded {}",
        ways.sequential.len(),
        ways.again.len()
    );

    eprintln!("{}", system());
    for (k, s) in ways.sequential.iter().enumerate() {
        let mut line = format!("FAER {} {} {:016x}", s.name, s.size, s.hash);
        for (t, items) in &ways.threads {
            let h = items[k].hash;
            if h == s.hash {
                line.push_str(&format!(" t{t}=same"));
            } else {
                line.push_str(&format!(" t{t}={h:016x}"));
            }
        }
        let h = ways.again[k].hash;
        if h == ways.threads[THREADS.len() - 1].1[k].hash {
            line.push_str(" again=same");
        } else {
            line.push_str(&format!(" again={h:016x}"));
        }
        eprintln!("{line}");
    }
    let dependent = ways.thread_dependent();
    eprintln!(
        "FAER-THREADS {} of {} items depend on the number of threads{}{}",
        dependent.len(),
        ways.sequential.len(),
        if dependent.is_empty() { "" } else { ": " },
        dependent.join(", ")
    );
    Some(ways)
}

/// The test's assertions, the same for every group: the same bits computed again, and the same
/// bits sequential and on every number of threads. faer's own QR, SVDs and eigenvectors failed
/// the second (its blocked Householder code divides its work by the threads: QR at 100 × 30 and
/// from 128 on, the SVDs through it, the eigenvectors at 128 on ARM, #832); the library's run
/// sequentially (`src/dense/fixed.rs`) and pass it.
pub(crate) fn assert_what_holds_on_one_system(ways: &Ways) {
    let again = ways.unrepeatable();
    assert!(
        again.is_empty(),
        "not the same bits computed again: {again:?}"
    );
    let threads = ways.thread_dependent();
    assert!(
        threads.is_empty(),
        "bits that depend on the number of threads: {threads:?}"
    );
}

//! Dynamic thermal compact models: heaters' responses in time as RC networks fitted to step
//! responses, after D. Coenen et al., "Analysis of thermal crosstalk in photonic integrated
//! circuit using dynamic compact models", IEEE Trans. Compon. Packag. Manuf. Technol. 12, 1350
//! (2022), doi:10.1109/TCPMT.2022.3195699. docs/methods/thermal-compact.md derives it.
//!
//! An [`RcModel`] maps the powers of its inputs (heaters, W) to its outputs (a temperature, or
//! a guide's phase) through a sum of first-order terms on poles every pair shares:
//!
//! Z_ij(s) = Σ_k R_k,ij / (1 + s τ_k), its step response Σ_k R_k,ij (1 − e^(−t/τ_k)).
//!
//! Each pair (i, j) is a Foster network ([`Foster`]), which [`Foster::cauer`] turns into the
//! equivalent ladder ([`Cauer`]). Every τ_k is positive, so the model is **stable by
//! construction**. Where the outputs are the heaters' own temperatures (driving points), a fit
//! with [`RcFit::passive`] makes every residue matrix symmetric and positive semidefinite, so
//! Re Z(iω) is too at every frequency: **passive** (the network gives out no heat it isn't
//! given), as a network of resistors and capacitors is. A guide's phase beside a heater isn't
//! a driving point: the heat takes time to reach it, which residues of both signs give (a sum
//! of decays of one sign starts at once), so a phase model is stable, and not made passive.

use num_complex::Complex64 as c64;

use crate::{Error, Result};

fn invalid(reason: impl Into<String>) -> Error {
    Error::invalid("thermal model", reason)
}

/// A dynamic thermal compact model: poles τ_k (s) shared by every pair of output and input, and
/// a residue matrix R_k per pole (`residues[k][output][input]`, the output's unit per W).
#[derive(Clone, Debug, PartialEq)]
pub struct RcModel {
    time_constants_s: Vec<f64>,
    residues: Vec<Vec<Vec<f64>>>,
}

/// How [`RcModel::fit`] fits.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct RcFit {
    /// How many poles (time constants), from 1.
    pub poles: usize,
    /// Whether a square model is made passive: each residue matrix symmetrized and its negative
    /// eigenvalues taken to 0 (false by default). For heaters' own temperatures; a model with
    /// more outputs than inputs, or fewer, is stable only.
    pub passive: bool,
}

impl RcFit {
    /// `poles` poles, stable (not made passive).
    pub fn new(poles: usize) -> RcFit {
        RcFit {
            poles,
            passive: false,
        }
    }

    /// Whether a square model is made passive.
    pub fn passive(mut self, passive: bool) -> RcFit {
        self.passive = passive;
        self
    }
}

/// A fitted model and its error against the step responses it was fitted to.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Fitted {
    /// The model.
    pub model: RcModel,
    /// The largest error over every pair and time, relative to the largest response of the
    /// pair's input (its own output's, for a heater and its guide).
    pub error: f64,
    /// The root mean square of the same relative errors.
    pub rms: f64,
}

impl RcModel {
    /// A model of poles `time_constants_s` and residue matrices `residues[k][output][input]`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no pole, a time constant that isn't finite and positive, a
    /// residue matrix per pole missing or of another shape than the first, an empty one, or a
    /// residue that isn't finite.
    pub fn new(time_constants_s: Vec<f64>, residues: Vec<Vec<Vec<f64>>>) -> Result<RcModel> {
        if time_constants_s.is_empty() {
            return Err(invalid("a model needs a pole"));
        }
        if let Some(t) = time_constants_s
            .iter()
            .find(|t| !(t.is_finite() && **t > 0.0))
        {
            return Err(invalid(format!(
                "a time constant must be finite and positive, not {t} s"
            )));
        }
        if residues.len() != time_constants_s.len() {
            return Err(invalid(format!(
                "{} poles need {} residue matrices, not {}",
                time_constants_s.len(),
                time_constants_s.len(),
                residues.len()
            )));
        }
        let outputs = residues[0].len();
        let inputs = residues[0].first().map_or(0, Vec::len);
        if outputs == 0 || inputs == 0 {
            return Err(invalid("a residue matrix needs an output and an input"));
        }
        for r in &residues {
            if r.len() != outputs || r.iter().any(|row| row.len() != inputs) {
                return Err(invalid(format!(
                    "every residue matrix must be {outputs} × {inputs}, as the first"
                )));
            }
            if r.iter().flatten().any(|v| !v.is_finite()) {
                return Err(invalid("a residue must be finite"));
            }
        }
        Ok(RcModel {
            time_constants_s,
            residues,
        })
    }

    /// The poles' time constants, seconds.
    pub fn time_constants_s(&self) -> &[f64] {
        &self.time_constants_s
    }

    /// The residue matrices, `[pole][output][input]`.
    pub fn residues(&self) -> &[Vec<Vec<f64>>] {
        &self.residues
    }

    /// How many outputs.
    pub fn outputs(&self) -> usize {
        self.residues[0].len()
    }

    /// How many inputs.
    pub fn inputs(&self) -> usize {
        self.residues[0][0].len()
    }

    /// The steady response, Z(0) = Σ_k R_k, `[output][input]`.
    pub fn steady(&self) -> Vec<Vec<f64>> {
        let mut z = vec![vec![0.0; self.inputs()]; self.outputs()];
        for r in &self.residues {
            for (zi, ri) in z.iter_mut().zip(r) {
                for (z, v) in zi.iter_mut().zip(ri) {
                    *z += v;
                }
            }
        }
        z
    }

    /// Output `output`'s step response to 1 W of input `input` at `time_s` after it is turned
    /// on: Σ_k R_k (1 − e^(−t/τ_k)).
    pub fn step(&self, output: usize, input: usize, time_s: f64) -> f64 {
        self.time_constants_s
            .iter()
            .zip(&self.residues)
            .map(|(t, r)| r[output][input] * -(-time_s / t).exp_m1())
            .sum()
    }

    /// Output `output`'s response to input `input`'s power at `frequency_hz`, per W:
    /// Σ_k R_k / (1 + i 2πf τ_k).
    pub fn response(&self, output: usize, input: usize, frequency_hz: f64) -> c64 {
        let w = 2.0 * std::f64::consts::PI * frequency_hz;
        self.time_constants_s
            .iter()
            .zip(&self.residues)
            .map(|(t, r)| r[output][input] / c64::new(1.0, w * t))
            .sum()
    }

    /// The −3 dB frequency of output `output`'s response to input `input`, Hz: the lowest where
    /// |Z(f)| falls to |Z(0)|/√2, by bisection in log f to 1e-9 of it.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the pair doesn't respond at all, or never falls that far
    /// between a thousandth of the slowest pole's frequency and a thousand times the fastest's.
    pub fn cutoff(&self, output: usize, input: usize) -> Result<f64> {
        let dc = self.response(output, input, 0.0).norm();
        if dc == 0.0 {
            return Err(invalid("the pair doesn't respond: no cutoff"));
        }
        let below =
            |f: f64| self.response(output, input, f).norm() < dc * std::f64::consts::FRAC_1_SQRT_2;
        let tau = |m: fn(f64, f64) -> f64| self.time_constants_s.iter().copied().fold(f64::NAN, m);
        let two_pi = 2.0 * std::f64::consts::PI;
        let (lo, hi) = (
            1e-3 / (two_pi * tau(f64::max)),
            1e3 / (two_pi * tau(f64::min)),
        );
        let n = 400;
        let at = |k: usize| lo * (hi / lo).powf(k as f64 / n as f64);
        let k = (1..=n)
            .find(|&k| below(at(k)))
            .ok_or_else(|| invalid("the response doesn't fall by 3 dB"))?;
        let (mut a, mut b) = (at(k - 1), at(k));
        while b / a - 1.0 > 1e-9 {
            let m = (a * b).sqrt();
            if below(m) {
                b = m;
            } else {
                a = m;
            }
        }
        Ok((a * b).sqrt())
    }

    /// The outputs at `times_s` from rest (no power before the first time) when input j's power
    /// is `powers[j][m]` W from `times_s[m]` to the next time, held: the exact solution of
    /// each pole's first-order equation for powers held over each step. Returns
    /// `[output][time]`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for times that aren't finite and increasing, or powers that
    /// aren't one list per input, each one per time.
    pub fn simulate(&self, times_s: &[f64], powers: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
        let (no, ni) = (self.outputs(), self.inputs());
        if powers.len() != ni || powers.iter().any(|p| p.len() != times_s.len()) {
            return Err(invalid(format!(
                "a simulation needs {ni} lists of powers, one per input, each one per time"
            )));
        }
        if times_s.iter().any(|t| !t.is_finite()) || times_s.windows(2).any(|w| w[1] <= w[0]) {
            return Err(invalid("the times must be finite and increasing"));
        }
        let np = self.time_constants_s.len();
        // x[k][j]: input j through pole k's first-order filter
        let mut x = vec![vec![0.0; ni]; np];
        let mut out = vec![vec![0.0; times_s.len()]; no];
        for m in 0..times_s.len() {
            for (i, o) in out.iter_mut().enumerate() {
                o[m] = (0..np)
                    .map(|k| {
                        (0..ni)
                            .map(|j| self.residues[k][i][j] * x[k][j])
                            .sum::<f64>()
                    })
                    .sum();
            }
            if let Some(&next) = times_s.get(m + 1) {
                let h = next - times_s[m];
                for (k, xk) in x.iter_mut().enumerate() {
                    let decay = (-h / self.time_constants_s[k]).exp();
                    for (j, v) in xk.iter_mut().enumerate() {
                        let u = powers[j][m];
                        *v = u + (*v - u) * decay;
                    }
                }
            }
        }
        Ok(out)
    }

    /// Whether the model is passive: square, every residue matrix symmetric and positive
    /// semidefinite (to 1e-12 of its largest entry).
    pub fn is_passive(&self) -> bool {
        let n = self.outputs();
        n == self.inputs()
            && self.residues.iter().all(|r| {
                let big = r.iter().flatten().fold(0.0f64, |a, v| a.max(v.abs()));
                let tol = 1e-12 * big.max(f64::MIN_POSITIVE);
                (0..n).all(|i| (0..n).all(|j| (r[i][j] - r[j][i]).abs() <= tol))
                    && eigen(r).0.iter().all(|&e| e >= -tol)
            })
    }

    /// Pair (output, input) as a Foster network.
    pub fn foster(&self, output: usize, input: usize) -> Foster {
        Foster {
            resistances: self.residues.iter().map(|r| r[output][input]).collect(),
            time_constants_s: self.time_constants_s.clone(),
        }
    }

    /// The model of `fit.poles` poles that best fits the step responses `steps[input][output]
    /// [time]` (from rest, per W) at `times_s`, through `steady[output][input]` at t → ∞ where
    /// it is given. See [`RcModel::fits`].
    ///
    /// # Errors
    ///
    /// As [`RcModel::fits`].
    pub fn fit(
        times_s: &[f64],
        steps: &[Vec<Vec<f64>>],
        steady: Option<&[Vec<f64>]>,
        fit: &RcFit,
    ) -> Result<Fitted> {
        let mut all = RcModel::fits(times_s, steps, steady, fit)?;
        all.pop().ok_or_else(|| invalid("a fit needs a pole"))
    }

    /// The models of 1 to `fit.poles` poles that best fit the step responses, each from the one
    /// before with a pole added: the error as poles are added.
    ///
    /// Each model's poles are found by variable projection: for given poles the residues are
    /// linear least squares, pair by pair (through the steady values exactly where they are
    /// given), and the poles (their logarithms) move by Levenberg–Marquardt to the least
    /// relative error over every pair, each pair's error relative to its input's largest
    /// response. A pole is added at each of a dozen places across the record and the best kept.
    /// A passive fit's residues are projected onto symmetric positive semidefinite matrices
    /// before each error is taken, so the poles are those that fit best once passive.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for no pole, fewer times than three per pole, times that aren't
    /// finite, positive (a time 0 may lead, its values 0) and increasing, responses that aren't
    /// one list per input of one list per output, each a value per time, or that are 0
    /// throughout for an input, or steady values of another shape.
    pub fn fits(
        times_s: &[f64],
        steps: &[Vec<Vec<f64>>],
        steady: Option<&[Vec<f64>]>,
        fit: &RcFit,
    ) -> Result<Vec<Fitted>> {
        let data = Data::new(times_s, steps, steady, fit)?;
        let (lo, hi) = (
            data.times[0].ln() - 2.0,
            data.times[data.times.len() - 1].ln() + 3.0,
        );
        let candidates: Vec<f64> = (0..12).map(|k| lo + (hi - lo) * k as f64 / 11.0).collect();
        let mut fits = Vec::with_capacity(fit.poles);
        let mut best: Vec<f64> = Vec::new();
        for _ in 0..fit.poles {
            let mut found: (f64, Vec<f64>) = (f64::INFINITY, Vec::new());
            for &c in &candidates {
                let mut start = best.clone();
                start.push(c);
                let (cost, theta) = data.refine(start, lo - 3.0, hi + 4.0);
                if found.1.is_empty() || cost < found.0 {
                    found = (cost, theta);
                }
            }
            let mut theta = found.1;
            theta.sort_by(f64::total_cmp);
            best = theta;
            fits.push(data.fitted(&best)?);
        }
        Ok(fits)
    }
}

/// A fit's data: times, the responses by pair, each pair's scale, the steady values.
struct Data {
    times: Vec<f64>,
    /// `[input][output][time]`.
    steps: Vec<Vec<Vec<f64>>>,
    steady: Option<Vec<Vec<f64>>>,
    /// Per input: its largest response.
    scale: Vec<f64>,
    passive: bool,
    outputs: usize,
}

impl Data {
    fn new(
        times_s: &[f64],
        steps: &[Vec<Vec<f64>>],
        steady: Option<&[Vec<f64>]>,
        fit: &RcFit,
    ) -> Result<Data> {
        if fit.poles == 0 {
            return Err(invalid("a fit needs a pole"));
        }
        let ni = steps.len();
        let no = steps.first().map_or(0, Vec::len);
        if ni == 0 || no == 0 {
            return Err(invalid("a fit needs an input and an output"));
        }
        let lead = usize::from(times_s.first() == Some(&0.0));
        let times = times_s[lead..].to_vec();
        if times.len() < 3 * fit.poles {
            return Err(invalid(format!(
                "{} poles need at least {} times, not {}",
                fit.poles,
                3 * fit.poles,
                times.len()
            )));
        }
        if times.iter().any(|t| !(t.is_finite() && *t > 0.0))
            || times.windows(2).any(|w| w[1] <= w[0])
        {
            return Err(invalid("the times must be finite, positive and increasing"));
        }
        let mut cut = Vec::with_capacity(ni);
        for s in steps {
            if s.len() != no || s.iter().any(|v| v.len() != times_s.len()) {
                return Err(invalid(format!(
                    "the responses must be {no} outputs per input, each a value per time"
                )));
            }
            if s.iter().flatten().any(|v| !v.is_finite()) {
                return Err(invalid("a response must be finite"));
            }
            cut.push(s.iter().map(|v| v[lead..].to_vec()).collect::<Vec<_>>());
        }
        let scale: Vec<f64> = cut
            .iter()
            .map(|s: &Vec<Vec<f64>>| s.iter().flatten().fold(0.0f64, |a, v| a.max(v.abs())))
            .collect();
        if scale.contains(&0.0) {
            return Err(invalid("an input's responses are 0 throughout"));
        }
        let steady = match steady {
            Some(z) => {
                if z.len() != no || z.iter().any(|r| r.len() != ni) {
                    return Err(invalid(format!(
                        "the steady values must be {no} × {ni}, outputs by inputs"
                    )));
                }
                if z.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(invalid("a steady value must be finite"));
                }
                Some(z.to_vec())
            }
            None => None,
        };
        Ok(Data {
            times,
            steps: cut,
            steady,
            scale,
            passive: fit.passive && no == ni,
            outputs: no,
        })
    }

    /// The residues `[pole][output][input]` for poles at ln τ = `theta`.
    #[allow(clippy::needless_range_loop)] // the small dense algebra reads best by index
    fn residues(&self, theta: &[f64]) -> Vec<Vec<Vec<f64>>> {
        let np = theta.len();
        let tau: Vec<f64> = theta.iter().map(|t| t.exp()).collect();
        let a: Vec<Vec<f64>> = self
            .times
            .iter()
            .map(|&t| tau.iter().map(|&s| -(-t / s).exp_m1()).collect())
            .collect();
        // the normal matrix, shared by every pair, with the steady row when there is one
        let n = np + usize::from(self.steady.is_some());
        let mut g = vec![vec![0.0; n]; n];
        for row in &a {
            for p in 0..np {
                for q in 0..np {
                    g[p][q] += row[p] * row[q];
                }
            }
        }
        let trace: f64 = (0..np).map(|p| g[p][p]).sum();
        for (p, gp) in g.iter_mut().enumerate().take(np) {
            gp[p] += 1e-13 * trace / np as f64;
        }
        if self.steady.is_some() {
            for p in 0..np {
                g[p][np] = 1.0;
                g[np][p] = 1.0;
            }
        }
        let ni = self.steps.len();
        let mut r = vec![vec![vec![0.0; ni]; self.outputs]; np];
        for j in 0..ni {
            for i in 0..self.outputs {
                let y = &self.steps[j][i];
                let mut b = vec![0.0; n];
                for (row, v) in a.iter().zip(y) {
                    for p in 0..np {
                        b[p] += row[p] * v;
                    }
                }
                if let Some(z) = &self.steady {
                    b[np] = z[i][j];
                }
                let x = solve(g.clone(), b);
                for p in 0..np {
                    r[p][i][j] = x[p];
                }
            }
        }
        if self.passive {
            for rk in &mut r {
                *rk = project(rk);
            }
        }
        r
    }

    /// The relative errors at every pair and time for poles `theta` and residues `r`.
    fn errors(&self, theta: &[f64], r: &[Vec<Vec<f64>>]) -> Vec<f64> {
        let tau: Vec<f64> = theta.iter().map(|t| t.exp()).collect();
        let mut e = Vec::with_capacity(self.steps.len() * self.outputs * self.times.len());
        for (j, s) in self.steps.iter().enumerate() {
            for (i, y) in s.iter().enumerate() {
                for (&t, v) in self.times.iter().zip(y) {
                    let m: f64 = tau
                        .iter()
                        .zip(r)
                        .map(|(&s, rk)| rk[i][j] * -(-t / s).exp_m1())
                        .sum();
                    e.push((m - v) / self.scale[j]);
                }
            }
        }
        e
    }

    fn residual(&self, theta: &[f64]) -> Vec<f64> {
        self.errors(theta, &self.residues(theta))
    }

    /// Levenberg–Marquardt on the poles' logarithms from `theta`, each kept between `lo` and
    /// `hi`: the cost (the squared errors' sum) and the poles.
    fn refine(&self, mut theta: Vec<f64>, lo: f64, hi: f64) -> (f64, Vec<f64>) {
        let cost = |r: &[f64]| r.iter().map(|v| v * v).sum::<f64>();
        let mut r = self.residual(&theta);
        let mut c = cost(&r);
        let mut mu = 1e-3;
        let n = theta.len();
        for _ in 0..200 {
            // the Jacobian by forward differences
            let h = 1e-6;
            let columns: Vec<Vec<f64>> = (0..n)
                .map(|p| {
                    let mut t = theta.clone();
                    t[p] += h;
                    self.residual(&t)
                        .iter()
                        .zip(&r)
                        .map(|(a, b)| (a - b) / h)
                        .collect()
                })
                .collect();
            let mut jtj = vec![vec![0.0; n]; n];
            let mut jtr = vec![0.0; n];
            for p in 0..n {
                for q in 0..n {
                    jtj[p][q] = columns[p].iter().zip(&columns[q]).map(|(a, b)| a * b).sum();
                }
                jtr[p] = columns[p].iter().zip(&r).map(|(a, b)| a * b).sum();
            }
            let mut improved = false;
            while mu < 1e12 {
                let mut m = jtj.clone();
                for (p, mp) in m.iter_mut().enumerate() {
                    mp[p] += mu * (jtj[p][p] + 1e-12);
                }
                let step = solve(m, jtr.iter().map(|v| -v).collect());
                let trial: Vec<f64> = theta
                    .iter()
                    .zip(&step)
                    .map(|(t, s)| (t + s).clamp(lo, hi))
                    .collect();
                let rt = self.residual(&trial);
                let ct = cost(&rt);
                if ct < c {
                    let done = c - ct <= 1e-12 * c;
                    theta = trial;
                    r = rt;
                    c = ct;
                    mu = (mu / 3.0).max(1e-12);
                    improved = !done;
                    break;
                }
                mu *= 4.0;
            }
            if !improved {
                break;
            }
        }
        (c, theta)
    }

    fn fitted(&self, theta: &[f64]) -> Result<Fitted> {
        let r = self.residues(theta);
        let e = self.errors(theta, &r);
        let error = e.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        let rms = (e.iter().map(|v| v * v).sum::<f64>() / e.len() as f64).sqrt();
        Ok(Fitted {
            model: RcModel::new(theta.iter().map(|t| t.exp()).collect(), r)?,
            error,
            rms,
        })
    }
}

/// Solves the small dense system m x = b by Gaussian elimination with partial pivoting; a zero
/// pivot leaves its unknown 0.
#[allow(clippy::needless_range_loop)] // elimination reads best by index
pub(crate) fn solve(mut m: Vec<Vec<f64>>, mut b: Vec<f64>) -> Vec<f64> {
    let n = b.len();
    for c in 0..n {
        let p = (c..n)
            .max_by(|&x, &y| m[x][c].abs().total_cmp(&m[y][c].abs()))
            .unwrap_or(c);
        m.swap(c, p);
        b.swap(c, p);
        if m[c][c] == 0.0 {
            continue;
        }
        for r in c + 1..n {
            let f = m[r][c] / m[c][c];
            if f != 0.0 {
                for k in c..n {
                    m[r][k] -= f * m[c][k];
                }
                b[r] -= f * b[c];
            }
        }
    }
    let mut x = vec![0.0; n];
    for c in (0..n).rev() {
        if m[c][c] == 0.0 {
            continue;
        }
        let s: f64 = (c + 1..n).map(|k| m[c][k] * x[k]).sum();
        x[c] = (b[c] - s) / m[c][c];
    }
    x
}

/// The eigenvalues and eigenvectors (columns) of the symmetric part of a small square matrix,
/// by cyclic Jacobi rotations.
#[allow(clippy::needless_range_loop)] // the rotations read best by index
fn eigen(a: &[Vec<f64>]) -> (Vec<f64>, Vec<Vec<f64>>) {
    let n = a.len();
    let mut m: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| 0.5 * (a[i][j] + a[j][i])).collect())
        .collect();
    let mut v: Vec<Vec<f64>> = (0..n)
        .map(|i| (0..n).map(|j| f64::from(u8::from(i == j))).collect())
        .collect();
    for _ in 0..100 {
        let off: f64 = (0..n)
            .flat_map(|i| (0..n).filter(move |&j| j != i).map(move |j| (i, j)))
            .map(|(i, j)| m[i][j] * m[i][j])
            .sum();
        let diag: f64 = (0..n).map(|i| m[i][i] * m[i][i]).sum();
        if off <= 1e-30 * diag.max(f64::MIN_POSITIVE) {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                if m[p][q] == 0.0 {
                    continue;
                }
                let theta = (m[q][q] - m[p][p]) / (2.0 * m[p][q]);
                let sign = if theta >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (theta.abs() + (theta * theta + 1.0).sqrt());
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (mkp, mkq) = (m[k][p], m[k][q]);
                    m[k][p] = c * mkp - s * mkq;
                    m[k][q] = s * mkp + c * mkq;
                }
                for k in 0..n {
                    let (mpk, mqk) = (m[p][k], m[q][k]);
                    m[p][k] = c * mpk - s * mqk;
                    m[q][k] = s * mpk + c * mqk;
                }
                for row in v.iter_mut() {
                    let (vp, vq) = (row[p], row[q]);
                    row[p] = c * vp - s * vq;
                    row[q] = s * vp + c * vq;
                }
            }
        }
    }
    ((0..n).map(|i| m[i][i]).collect(), v)
}

/// The nearest symmetric positive semidefinite matrix to a square one (in the Frobenius norm):
/// its symmetric part with the negative eigenvalues taken to 0.
fn project(a: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = a.len();
    let (e, v) = eigen(a);
    (0..n)
        .map(|i| {
            (0..n)
                .map(|j| (0..n).map(|k| e[k].max(0.0) * v[i][k] * v[j][k]).sum())
                .collect()
        })
        .collect()
}

/// A Foster network: resistances R_k (K/W, or the output's unit per W) each in parallel with a
/// capacitance C_k = τ_k/R_k, the blocks in series: Z(s) = Σ R_k/(1 + s τ_k). Coenen et al.'s
/// least-squares fit of a step response; its blocks are a fit's, not places in the structure.
#[derive(Clone, Debug, PartialEq)]
pub struct Foster {
    resistances: Vec<f64>,
    time_constants_s: Vec<f64>,
}

/// A Cauer ladder: the input's node has C₁ to the ambient and R₁ on to the next node, which has
/// C₂ and R₂ on, and so on, R_n to the ambient: Z = 1/(sC₁ + 1/(R₁ + 1/(sC₂ + … + 1/R_n))).
/// Its resistances and capacitances follow the heat's path from the source outward, which is
/// why Coenen et al. convert to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Cauer {
    resistances: Vec<f64>,
    capacitances: Vec<f64>,
}

impl Foster {
    /// A Foster network of resistances and time constants (s).
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for none, lists of different lengths, a resistance that isn't
    /// finite, or a time constant that isn't finite and positive.
    pub fn new(resistances: Vec<f64>, time_constants_s: Vec<f64>) -> Result<Foster> {
        if resistances.is_empty() || resistances.len() != time_constants_s.len() {
            return Err(invalid(
                "a Foster network needs as many time constants as resistances, at least one",
            ));
        }
        if resistances.iter().any(|r| !r.is_finite())
            || time_constants_s
                .iter()
                .any(|t| !(t.is_finite() && *t > 0.0))
        {
            return Err(invalid(
                "a Foster network's resistances must be finite, its time constants positive",
            ));
        }
        Ok(Foster {
            resistances,
            time_constants_s,
        })
    }

    /// A Foster network of resistances and capacitances (τ_k = R_k C_k), as Coenen et al.'s
    /// Table I gives them.
    ///
    /// # Errors
    ///
    /// As [`Foster::new`].
    pub fn of_rc(resistances: Vec<f64>, capacitances: Vec<f64>) -> Result<Foster> {
        let tau = resistances
            .iter()
            .zip(&capacitances)
            .map(|(r, c)| r * c)
            .collect();
        if resistances.len() != capacitances.len() {
            return Err(invalid(
                "a Foster network needs a capacitance per resistance",
            ));
        }
        Foster::new(resistances, tau)
    }

    /// The resistances.
    pub fn resistances(&self) -> &[f64] {
        &self.resistances
    }

    /// The time constants, seconds.
    pub fn time_constants_s(&self) -> &[f64] {
        &self.time_constants_s
    }

    /// The capacitances, τ_k/R_k.
    pub fn capacitances(&self) -> Vec<f64> {
        self.time_constants_s
            .iter()
            .zip(&self.resistances)
            .map(|(t, r)| t / r)
            .collect()
    }

    /// Z(i 2πf).
    pub fn impedance(&self, frequency_hz: f64) -> c64 {
        let w = 2.0 * std::f64::consts::PI * frequency_hz;
        self.resistances
            .iter()
            .zip(&self.time_constants_s)
            .map(|(r, t)| r / c64::new(1.0, w * t))
            .sum()
    }

    /// The equivalent Cauer ladder, Z(s) the same: Z = N(s)/D(s) as polynomials (Coenen et
    /// al.'s Eq. 3), and 1/Z expanded in a continued fraction from s → ∞ (their ref. 18), each
    /// step a capacitance then a resistance. Time is scaled by the slowest pole for the
    /// polynomials' conditioning.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a resistance isn't positive (a negative block has no ladder)
    /// or the expansion gives a ladder element that isn't positive (poles too close for the
    /// polynomials in f64).
    pub fn cauer(&self) -> Result<Cauer> {
        if self.resistances.iter().any(|r| *r <= 0.0) {
            return Err(invalid(
                "a Foster network with a resistance that isn't positive has no Cauer ladder",
            ));
        }
        let scale = self.time_constants_s.iter().copied().fold(0.0, f64::max);
        let tau: Vec<f64> = self.time_constants_s.iter().map(|t| t / scale).collect();
        // D = Π (1 + s τ_l), N = Σ R_k Π_{l≠k} (1 + s τ_l), coefficients from s⁰ up
        let times = |p: &[f64], t: f64| {
            let mut out = vec![0.0; p.len() + 1];
            for (k, c) in p.iter().enumerate() {
                out[k] += c;
                out[k + 1] += c * t;
            }
            out
        };
        let mut d = vec![1.0];
        for &t in &tau {
            d = times(&d, t);
        }
        let mut num = vec![0.0; tau.len()];
        for (k, r) in self.resistances.iter().enumerate() {
            let mut p = vec![*r];
            for (l, &t) in tau.iter().enumerate() {
                if l != k {
                    p = times(&p, t);
                }
            }
            for (a, b) in num.iter_mut().zip(&p) {
                *a += b;
            }
        }
        // Y = p/q, deg p = deg q + 1
        let (mut p, mut q) = (d, num);
        let (mut rs, mut cs) = (Vec::new(), Vec::new());
        let bad = || invalid("the Cauer expansion gave an element that isn't positive");
        loop {
            // C = p's leading coefficient over q's; p ← p − C s q
            let c = p[p.len() - 1] / q[q.len() - 1];
            for (k, v) in q.iter().enumerate() {
                p[k + 1] -= c * v;
            }
            p.pop();
            // now deg p = deg q; R = q's leading over p's; q ← q − R p
            let r = q[q.len() - 1] / p[p.len() - 1];
            if !(c > 0.0 && r > 0.0 && c.is_finite() && r.is_finite()) {
                return Err(bad());
            }
            cs.push(c * scale);
            rs.push(r);
            for (k, v) in p.iter().enumerate() {
                q[k] -= r * v;
            }
            q.pop();
            if q.is_empty() {
                break;
            }
            if rs.len() > self.resistances.len() {
                return Err(bad());
            }
        }
        if rs.len() != self.resistances.len() {
            return Err(bad());
        }
        Ok(Cauer {
            resistances: rs,
            capacitances: cs,
        })
    }
}

impl Cauer {
    /// The ladder's resistances, from the input.
    pub fn resistances(&self) -> &[f64] {
        &self.resistances
    }

    /// The ladder's capacitances, from the input.
    pub fn capacitances(&self) -> &[f64] {
        &self.capacitances
    }

    /// Z(i 2πf), the ladder evaluated from its far end.
    pub fn impedance(&self, frequency_hz: f64) -> c64 {
        let s = c64::new(0.0, 2.0 * std::f64::consts::PI * frequency_hz);
        let n = self.resistances.len();
        let mut z = c64::new(self.resistances[n - 1], 0.0);
        let mut y = c64::new(0.0, 0.0);
        for k in (0..n).rev() {
            y = s * self.capacitances[k] + 1.0 / z;
            if k > 0 {
                z = self.resistances[k - 1] + 1.0 / y;
            }
        }
        1.0 / y
    }
}

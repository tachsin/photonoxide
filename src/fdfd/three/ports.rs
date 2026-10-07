//! Ports in 3D: waveguide modes in and out of a 3D FDFD problem, and its S-parameters.
//!
//! A port is a plane of nodes normal to an axis a, crossing a waveguide that runs along a. On
//! the plane lie the tangential components E_b and E_c ((a, b, c) cyclic), and on the half-plane
//! after it the normal component E_a. Its **modes** are the scheme's own: a field that goes as
//! e^(iβa) on the grid turns every difference along a into iβ_d, β_d = (2/Δa) sin(βΔa/2), and
//! the discrete continuity equation, ∇ · (εE) = 0 at every node of the plane, gives E_a from
//! the tangential field, E_a = i (∇_t · (ε E_t)) / (β_d ε_a). What is left of the curl-curl
//! equation on the plane is an eigenproblem for (E_b, E_c), linear in β_d²:
//!
//! (−∇_t × ∇_t × + ∇_t ε_a⁻¹ ∇_t · ε + k₀² ε) E_t = β_d² E_t,
//!
//! every difference the 3D grid's own (the PMLs' stretch included), so a mode solved this way
//! satisfies every row of the 3D system along a straight grid waveguide exactly. This is the
//! transverse-E formulation of a vector mode solver, here on the Yee plane.
//!
//! **Sources** are total-field/scattered-field, as in 2D (R. C. Rumpf, Prog. Electromagn. Res. B
//! 36, 221 (2012), doi:10.2528/PIERB11092006, Eq. 55): b = (QA − AQ) f, with Q masking the
//! values on the scattered-field side of the plane and f the mode extended along a.
//!
//! **Amplitudes** come from the scheme's unconjugated Lorentz form. With V the product of the
//! PMLs' stretches at each value of E, V A is symmetric, and for two fields u and w the form
//!
//! Λ(u, w) = Σ (V A)_rs (w_r u_s − u_r w_s), r before the cut and s after it,
//!
//! taken across the cut between the port's plane and the next, is the same across every cut of
//! a stretch of guide without sources, when u and w both solve it there. So it vanishes between
//! two modes unless one is the other going the other way: that is the modes' orthogonality.
//! Each mode is normalized to N(f, g) = Δa Δb Δc Λ(f, g) / (4 i k₀) = 1, f the mode going
//! forward and g backward. N is the mode's power when the mode is real (a lossless guide clear
//! of the PMLs), and its unconjugated version otherwise. A field's forward and backward
//! amplitudes in the mode are then N(u, g) and −N(u, f), and an S-matrix from them is
//! power-normalized and symmetric for a reciprocal device.

use std::ops::Range;

use num_complex::Complex64 as c64;

use super::{Axis, Field3d, Lattice};
use crate::fdfd::{Direction, Edges, Side};
use crate::{Error, Result};

/// A mode of a 3D port: a waveguide's cross-section on a plane of nodes normal to an axis.
#[derive(Clone, Debug, PartialEq)]
pub struct PortMode3d {
    axis: Axis,
    plane: usize,
    /// The cells along the plane's two axes, `axis.others()`.
    size: [usize; 2],
    /// The window, along the same two axes.
    window: [Range<usize>; 2],
    k0: f64,
    step: f64,
    beta: c64,
    /// E_b, then E_c, on the plane: `size[0]` × `size[1]` values each, b fastest.
    tangential: Vec<c64>,
    /// E_a on the half-plane after the plane, the mode going forward.
    normal: Vec<c64>,
    power: f64,
}

/// A port of a 3D device: a mode, and the side of the device it is on, along the mode's axis.
#[derive(Clone, Debug, PartialEq)]
pub struct Port3d {
    /// The mode in and out.
    pub mode: PortMode3d,
    /// Its side: [`Side::Left`] at the low end of the axis (light comes in towards +axis),
    /// [`Side::Right`] at the high end.
    pub side: Side,
}

impl Port3d {
    fn incoming(&self) -> Direction {
        match self.side {
            Side::Left => Direction::Forward,
            Side::Right => Direction::Backward,
        }
    }
}

impl PortMode3d {
    /// The axis the waveguide runs along, normal to the port's plane.
    pub fn axis(&self) -> Axis {
        self.axis
    }

    /// The plane of nodes along [`PortMode3d::axis`] the mode was solved on; its amplitudes are
    /// measured across this plane and the next.
    pub fn plane(&self) -> usize {
        self.plane
    }

    /// The propagation constant β, rad/µm.
    pub fn beta(&self) -> c64 {
        self.beta
    }

    /// The effective index, β/k₀.
    pub fn effective_index(&self) -> c64 {
        self.beta / self.k0
    }

    /// The power the mode carries forward at unit amplitude (η₀ = 1, in |field|² µm²): the
    /// scheme's flux ([`Field3d::flux`]) between the plane and the next. 1 for a real mode,
    /// whose normalization is its power.
    pub fn power(&self) -> f64 {
        self.power
    }

    /// The mode's E, going forward at unit amplitude: its `component` at the plane's (u, v),
    /// the indices along the plane's two axes, `axis.others()`. The tangential components are
    /// on the plane, and the normal one on the half-plane after it (as [`super::Grid3d::index`]
    /// places it). The tangential field is real for a lossless guide, and the normal one 90°
    /// out of phase with it.
    pub fn e(&self, component: Axis, (u, v): (usize, usize)) -> c64 {
        let (b, _) = self.axis.others();
        let k = v * self.size[0] + u;
        if component == self.axis {
            self.normal[k]
        } else if component == b {
            self.tangential[k]
        } else {
            self.tangential[self.size[0] * self.size[1] + k]
        }
    }

    /// The wavenumber k₀ = ω/c the mode was solved at, rad/µm.
    pub(crate) fn k0(&self) -> f64 {
        self.k0
    }

    /// The cells along the plane's two axes, `axis.others()`, and the step along the axis.
    pub(crate) fn shape(&self) -> ([usize; 2], f64) {
        (self.size, self.step)
    }

    /// The mode's value `r` of the 3D field, going `direction` at unit amplitude on the plane.
    pub(super) fn value(&self, lattice: &Lattice, r: usize, direction: Direction) -> c64 {
        let (component, at) = lattice.grid.at(r);
        self.value_at(component, at, direction)
    }

    /// The mode's `component` at the 3D index `at`, going `direction` at unit amplitude on the
    /// plane: its profile on the plane, carried along the axis as e^(±iβΔa) per step.
    pub(crate) fn value_at(&self, component: Axis, at: [usize; 3], direction: Direction) -> c64 {
        let (b, c) = self.axis.others();
        let (u, v, m) = (at[b.index()], at[c.index()], at[self.axis.index()]);
        let steps = m as f64 - self.plane as f64;
        let i_beta = c64::new(0.0, 1.0) * self.beta * self.step;
        match direction {
            Direction::Forward => self.e(component, (u, v)) * (i_beta * steps).exp(),
            Direction::Backward => {
                let phase = (-i_beta * steps).exp();
                if component == self.axis {
                    // E_a changes sign with β: −e_a e^(−iβΔa/2) on the half-plane after
                    -self.e(component, (u, v)) * (-i_beta).exp() * phase
                } else {
                    self.e(component, (u, v)) * phase
                }
            }
        }
    }
}

/// Which values of E on a port's plane, and on the half-plane after it, belong to its window.
struct Window<'a> {
    lattice: &'a Lattice,
    axis: Axis,
    plane: usize,
    ranges: [Range<usize>; 2],
}

impl Window<'_> {
    /// Whether a value at index `m` along the window's axis `w` (0: b, 1: c), on a node or
    /// halfway after it, is inside: a node strictly inside the window, so that the tangential
    /// field is zero on its walls, or any half-step within it. A Bloch-periodic axis is whole.
    fn along(&self, w: usize, m: usize, half: bool) -> bool {
        let axis = [self.axis.others().0, self.axis.others().1][w];
        let range = &self.ranges[w];
        if matches!(self.lattice.boundaries.edges(axis), Edges::Bloch { .. }) {
            return true;
        }
        if half {
            range.contains(&m)
        } else {
            m > range.start && m < range.end
        }
    }

    /// Whether E's value `r` is on the plane (tangential), or on the half-plane after it
    /// (normal), and inside the window.
    fn contains(&self, r: usize) -> bool {
        let (component, at) = self.lattice.grid.at(r);
        if at[self.axis.index()] != self.plane {
            return false;
        }
        let (b, c) = self.axis.others();
        let (u, v) = (at[b.index()], at[c.index()]);
        self.along(0, u, component == b) && self.along(1, v, component == c)
    }
}

impl Lattice {
    /// The product of the PML's stretches along the three axes at E's value `r`: the weight V
    /// that makes V A symmetric.
    fn volume(&self, r: usize) -> c64 {
        let (component, at) = self.grid.at(r);
        Axis::ALL
            .into_iter()
            .map(|axis| {
                let m = at[axis.index()];
                if axis == component {
                    self.halves[axis.index()][m]
                } else {
                    self.nodes[axis.index()][m]
                }
            })
            .product()
    }

    /// The 3D index of `component` at (u, v) on the plane `plane` normal to `axis`.
    fn on_plane(&self, axis: Axis, plane: usize, component: Axis, (u, v): (usize, usize)) -> usize {
        let (b, c) = axis.others();
        let mut at = [0; 3];
        at[axis.index()] = plane;
        at[b.index()] = u;
        at[c.index()] = v;
        self.grid.index(component, (at[0], at[1], at[2]))
    }

    /// The Lorentz form N(u, w) = Δa Δb Δc Λ(u, w) / (4 i k₀) across the cut between nodes
    /// `plane` and `plane + 1` along `axis`, for the fields whose value `r` is `u(r)` and `w(r)`
    /// (see the module's docs).
    fn lorentz(
        &self,
        (axis, plane): (Axis, usize),
        u: &impl Fn(usize) -> c64,
        w: &impl Fn(usize) -> c64,
    ) -> c64 {
        let g = self.grid;
        let (b, c) = axis.others();
        let mut total = c64::new(0.0, 0.0);
        let mut row = Vec::with_capacity(32);
        for v in 0..g.n(c) {
            for uu in 0..g.n(b) {
                // the values before the cut that couple across it: the tangential E on the
                // plane, and the normal E on the half-plane after it
                for component in [b, c, axis] {
                    let r = self.on_plane(axis, plane, component, (uu, v));
                    let (ur, wr) = (u(r), w(r));
                    row.clear();
                    self.curl_curl_into(r, &mut row);
                    let mut sum = c64::new(0.0, 0.0);
                    for &(s, a) in &row {
                        let (to, at) = g.at(s);
                        if to != axis && at[axis.index()] == plane + 1 {
                            sum += a * (wr * u(s) - ur * w(s));
                        }
                    }
                    total += self.volume(r) * sum;
                }
            }
        }
        let volume = g.dx * g.dy * g.dz;
        total * volume / c64::new(0.0, 4.0 * self.k0)
    }

    /// Checks that a port on `plane` normal to `axis`, with `window`, fits the grid.
    fn check_port(&self, axis: Axis, plane: usize, window: &[Range<usize>; 2]) -> Result<()> {
        let g = self.grid;
        let (low, high) = match self.boundaries.edges(axis) {
            Edges::Pml { low, high } => (low, high),
            Edges::Bloch { .. } => {
                return Err(Error::invalid(
                    "fdfd port",
                    format!(
                        "a port normal to {axis:?} needs PMLs along it, not Bloch-periodic ends"
                    ),
                ));
            }
        };
        let n = g.n(axis);
        if plane < low + 2 || plane + 3 + high > n {
            return Err(Error::invalid(
                "fdfd port",
                format!(
                    "plane {plane} must be two cells clear of the PMLs ({low} and {high} cells) \
                     and the ends of {n} cells along {axis:?}"
                ),
            ));
        }
        let (b, c) = axis.others();
        for (w, along) in [b, c].into_iter().enumerate() {
            let range = &window[w];
            let cells = g.n(along);
            match self.boundaries.edges(along) {
                Edges::Bloch { k } => {
                    if k != 0.0 || *range != (0..cells) {
                        return Err(Error::invalid(
                            "fdfd port",
                            format!(
                                "a Bloch-periodic {along:?} must be periodic (k = 0) and the \
                                 window whole along it"
                            ),
                        ));
                    }
                }
                Edges::Pml { .. } => {
                    if range.end > cells || range.len() < 3 {
                        return Err(Error::invalid(
                            "fdfd port",
                            format!(
                                "the window along {along:?}, cells {} to {}, must have 3 cells or \
                                 more on the grid's {cells}",
                                range.start, range.end
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// The `count` modes of the guide crossing `plane` normal to `axis`, within `window`,
    /// highest effective index first (see the module's docs).
    pub(crate) fn port_modes(
        &self,
        eps: &[c64],
        (axis, plane): (Axis, usize),
        window: [Range<usize>; 2],
        count: usize,
    ) -> Result<Vec<PortMode3d>> {
        self.check_port(axis, plane, &window)?;
        let g = self.grid;
        let (b, c) = axis.others();
        let size = [g.n(b), g.n(c)];
        let inside = Window {
            lattice: self,
            axis,
            plane,
            ranges: window.clone(),
        };
        // the unknowns: E_b and E_c inside the window, numbered
        let per = size[0] * size[1];
        let mut local = vec![usize::MAX; 2 * per];
        let mut unknowns = Vec::new();
        for (t, component) in [b, c].into_iter().enumerate() {
            for v in 0..size[1] {
                for u in 0..size[0] {
                    let r = self.on_plane(axis, plane, component, (u, v));
                    if inside.contains(r) && !self.fixed(r) {
                        local[t * per + v * size[0] + u] = unknowns.len();
                        unknowns.push(r);
                    }
                }
            }
        }
        if unknowns.is_empty() {
            return Err(Error::invalid("fdfd port", "the window holds no field"));
        }
        let number = |r: usize| -> Option<usize> {
            let (component, at) = g.at(r);
            if component == axis || at[axis.index()] != plane {
                return None;
            }
            let t = usize::from(component == c);
            let k = local[t * per + at[c.index()] * size[0] + at[b.index()]];
            (k != usize::MAX).then_some(k)
        };
        // ε_a at a node of the plane: the normal E's own, on the half-plane after it
        let normal_eps = |q: usize| {
            let (_, at) = g.at(q);
            eps[g.index(axis, (at[0], at[1], at[2]))]
        };
        let node_inside = |q: usize| {
            let (_, at) = g.at(q);
            inside.contains(g.index(axis, (at[0], at[1], at[2])))
        };
        let k2 = self.k0 * self.k0;
        let mut entries = Vec::with_capacity(16 * unknowns.len());
        let mut top: f64 = 1.0;
        for (row, &r) in unknowns.iter().enumerate() {
            top = top.max(eps[r].re);
            entries.push((row, row, k2 * eps[r]));
            // −∇_t × ∇_t × E_t: through H_a, on the faces of the plane
            for (face, wh) in self.curl_h(r) {
                if g.at(face).0 != axis {
                    continue;
                }
                for (col, we) in self.curl_e(face) {
                    if let Some(k) = number(col) {
                        entries.push((row, k, -wh * we));
                    }
                }
            }
            // ∇_t (ε_a⁻¹ ∇_t · (ε E_t)), the continuity equation's E_a
            for (q, wg) in self.gradient(r) {
                if !node_inside(q) {
                    continue;
                }
                let inverse = 1.0 / normal_eps(q);
                for (col, wd) in self.divergence(q) {
                    if let Some(k) = number(col) {
                        entries.push((row, k, wg * wd * eps[col] * inverse));
                    }
                }
            }
        }
        let shift = c64::new(k2 * top, 0.0);
        let pairs = crate::eigen::nearest(unknowns.len(), &entries, shift, count, 1e-10)?;
        let h = g.step(axis);
        let mut modes = Vec::with_capacity(pairs.len());
        for pair in pairs {
            let at_plane =
                |r: usize| -> c64 { number(r).map_or(c64::new(0.0, 0.0), |k| pair.vector[k]) };
            let build = |beta_d: c64| -> PortMode3d {
                let beta = 2.0 / h * (beta_d * h / 2.0).asin();
                let mut tangential = vec![c64::new(0.0, 0.0); 2 * per];
                for (t, &k) in local.iter().enumerate() {
                    if k != usize::MAX {
                        tangential[t] = pair.vector[k];
                    }
                }
                // E_a = i ∇_t · (ε E_t) / (β_d ε_a), on the half-plane after: e^(iβΔa/2) on
                let half = (c64::new(0.0, 0.5) * beta * h).exp();
                let mut normal = vec![c64::new(0.0, 0.0); per];
                for v in 0..size[1] {
                    for u in 0..size[0] {
                        let r = self.on_plane(axis, plane, axis, (u, v));
                        if !inside.contains(r) || self.fixed(r) {
                            continue;
                        }
                        let q = self.on_plane(axis, plane, Axis::X, (u, v));
                        let divergence: c64 = self
                            .divergence(q)
                            .into_iter()
                            .filter(|&(col, _)| g.at(col).0 != axis)
                            .map(|(col, wd)| wd * eps[col] * at_plane(col))
                            .sum();
                        normal[v * size[0] + u] =
                            c64::new(0.0, 1.0) * divergence / (beta_d * eps[r]) * half;
                    }
                }
                let mut mode = PortMode3d {
                    axis,
                    plane,
                    size,
                    window: window.clone(),
                    k0: self.k0,
                    step: h,
                    beta,
                    tangential,
                    normal,
                    power: 0.0,
                };
                // N(f, g) = 1: real for a lossless guide; then the largest value positive
                let norm = self.lorentz(
                    (axis, plane),
                    &|r| mode.value(self, r, Direction::Forward),
                    &|r| mode.value(self, r, Direction::Backward),
                );
                let mut scale = 1.0 / norm.sqrt();
                let peak = mode
                    .tangential
                    .iter()
                    .copied()
                    .max_by(|p, q| p.norm().total_cmp(&q.norm()))
                    .unwrap_or(c64::new(1.0, 0.0));
                if (peak * scale).re < 0.0 {
                    scale = -scale;
                }
                mode.tangential.iter_mut().for_each(|x| *x *= scale);
                mode.normal.iter_mut().for_each(|x| *x *= scale);
                mode.power =
                    self.flux_with(axis, plane, &|r| mode.value(self, r, Direction::Forward));
                mode
            };
            // forward, for a mode that propagates more than it decays, is the way its power
            // goes, which a backward wave (in a closed, inhomogeneous guide) takes against its
            // phase; for one that decays more, the way it decays. A guided mode whose plane
            // crosses PMLs has a small Im β from them, of either sign (a PML doesn't absorb an
            // evanescent tail): it propagates all the same, and taken as decaying, half of them
            // went backward
            let beta_d = pair.value.sqrt();
            let propagating = propagates(beta_d);
            let beta_d = if !propagating && beta_d.im < 0.0 || propagating && beta_d.re < 0.0 {
                -beta_d
            } else {
                beta_d
            };
            let mut mode = build(beta_d);
            if propagating && mode.power < 0.0 {
                mode = build(-beta_d);
            }
            modes.push(mode);
        }
        // propagating modes first, by effective index, then the others by their decay
        modes.sort_by(|p, q| {
            let key = |m: &PortMode3d| (!propagates(m.beta), m.beta.im);
            let (kp, kq) = (key(p), key(q));
            kp.0.cmp(&kq.0)
                .then(q.beta.re.abs().total_cmp(&p.beta.re.abs()))
                .then(kp.1.total_cmp(&kq.1))
        });
        Ok(modes)
    }

    /// The right-hand side of A E = b that launches `mode` at unit amplitude going `direction`,
    /// by total-field/scattered-field: going forward the total field is from the mode's plane
    /// on, going backward up to the plane after it.
    pub(crate) fn mode_source(&self, mode: &PortMode3d, direction: Direction) -> Vec<c64> {
        let g = self.grid;
        let axis = mode.axis;
        let p = mode.plane as i64;
        // twice the position along the axis, in steps: a node is 2m, the half-step after 2m + 1
        let scattered = |r: usize| {
            let (component, at) = g.at(r);
            let twice = 2 * at[axis.index()] as i64 + i64::from(component == axis);
            match direction {
                Direction::Forward => twice < 2 * p,
                Direction::Backward => twice > 2 * (p + 1),
            }
        };
        let mut b = vec![c64::new(0.0, 0.0); g.unknowns()];
        let (lo, hi) = (mode.plane - 1, mode.plane + 2);
        let mut row = Vec::with_capacity(32);
        let (bb, cc) = axis.others();
        for m in lo..=hi {
            for v in 0..g.n(cc) {
                for u in 0..g.n(bb) {
                    for component in Axis::ALL {
                        let r = self.on_plane(axis, m, component, (u, v));
                        row.clear();
                        self.curl_curl_into(r, &mut row);
                        let qr = scattered(r);
                        // (QA − AQ)_rs = (Q_r − Q_s) A_rs
                        let sum: c64 = row
                            .iter()
                            .filter(|&&(s, _)| scattered(s) != qr)
                            .map(|&(s, a)| {
                                let sign = if qr { 1.0 } else { -1.0 };
                                sign * a * mode.value(self, s, direction)
                            })
                            .sum();
                        b[r] += sum;
                    }
                }
            }
        }
        b
    }

    /// The forward and backward amplitudes of `mode` in the field whose value `r` is `u(r)`.
    pub(crate) fn mode_amplitudes(
        &self,
        mode: &PortMode3d,
        u: &impl Fn(usize) -> c64,
    ) -> (c64, c64) {
        assert!(
            self.fits(mode),
            "the mode must come from this problem's port_modes"
        );
        let cut = (mode.axis, mode.plane);
        let forward = self.lorentz(cut, u, &|r| mode.value(self, r, Direction::Backward));
        let backward = -self.lorentz(cut, u, &|r| mode.value(self, r, Direction::Forward));
        (forward, backward)
    }

    /// Whether `mode` can be a mode of this lattice: its plane's size, step and wavenumber.
    fn fits(&self, mode: &PortMode3d) -> bool {
        let (b, c) = mode.axis.others();
        mode.size == [self.grid.n(b), self.grid.n(c)]
            && mode.k0 == self.k0
            && mode.step == self.grid.step(mode.axis)
            && mode.plane + 1 < self.grid.n(mode.axis)
    }

    /// The S-matrix between `ports`, from `solve`, which gives the field for a right-hand side.
    pub(crate) fn s_matrix(
        &self,
        ports: &[Port3d],
        mut solve: impl FnMut(&[c64]) -> Result<Field3d>,
    ) -> Result<Vec<Vec<c64>>> {
        use faer::linalg::solvers::Solve;
        for p in ports {
            if !self.fits(&p.mode) {
                return Err(Error::invalid(
                    "fdfd port",
                    "a port's mode must come from this problem's port_modes",
                ));
            }
        }
        let n = ports.len();
        let mut incoming = faer::Mat::<c64>::zeros(n, n);
        let mut outgoing = faer::Mat::<c64>::zeros(n, n);
        for (p, port) in ports.iter().enumerate() {
            let field = solve(&self.mode_source(&port.mode, port.incoming()))?;
            for (q, at) in ports.iter().enumerate() {
                let (forward, backward) = self.mode_amplitudes(&at.mode, &|r| field.values[r]);
                let (a, b) = match at.side {
                    Side::Left => (forward, backward),
                    Side::Right => (backward, forward),
                };
                incoming[(q, p)] = a;
                outgoing[(q, p)] = b;
            }
        }
        // S A = B, so Aᵀ Sᵀ = Bᵀ
        let st = incoming
            .transpose()
            .to_owned()
            .partial_piv_lu()
            .solve(outgoing.transpose().to_owned());
        Ok((0..n)
            .map(|q| (0..n).map(|p| st[(p, q)]).collect())
            .collect())
    }
}

/// Whether a mode with this β propagates more than it decays: |Im β| < |Re β|.
fn propagates(beta: c64) -> bool {
    beta.im.abs() < beta.re.abs()
}

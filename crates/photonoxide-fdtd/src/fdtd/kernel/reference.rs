//! The plain loops the kernel replaced, kept as its reference: the fast kernel must give the
//! same bits in f64.

use rayon::prelude::*;

use super::super::{Field, Simulation};
use crate::fdfd::{Axis, Grid3d};

impl Simulation {
    /// One component's update: `out = keep out + scale (D_a f_b − D_b f_a)`, D the difference
    /// along an axis (forward for H's updates, back for E's) times 1/(κ Δ) there; without
    /// `keep` and `scale`, `out −= Δt (D_a f_b − D_b f_a)` (H's update).
    #[allow(clippy::too_many_arguments)]
    fn curl_update_reference(
        grid: Grid3d,
        out: &mut [f64],
        component: Axis,
        (fa, fb): (&[f64], &[f64]),
        offsets: &[Vec<Option<isize>>; 3],
        factors: &[Vec<f64>; 3],
        coefficients: Option<(&[f64], &[f64])>,
        dt: f64,
    ) {
        let (a, b) = component.others();
        let plane = grid.nx * grid.ny;
        let forward = coefficients.is_none();
        let difference = |f: &[f64], r: usize, offset: Option<isize>| -> f64 {
            let other = offset.map_or(0.0, |o| f[(r as isize + o) as usize]);
            if forward { other - f[r] } else { f[r] - other }
        };
        out.par_chunks_mut(plane)
            .enumerate()
            .for_each(|(k, values)| {
                for j in 0..grid.ny {
                    for i in 0..grid.nx {
                        let m = [i, j, k];
                        let r = k * plane + j * grid.nx + i;
                        let (ma, mb) = (m[a.index()], m[b.index()]);
                        let curl = difference(fb, r, offsets[a.index()][ma])
                            * factors[a.index()][ma]
                            - difference(fa, r, offsets[b.index()][mb]) * factors[b.index()][mb];
                        let v = &mut values[j * grid.nx + i];
                        match coefficients {
                            None => *v -= dt * curl,
                            Some((keep, scale)) => *v = keep[r] * *v + scale[r] * curl,
                        }
                    }
                }
            });
    }

    pub(crate) fn update_h_reference(&mut self) {
        let offsets = self.offsets(true);
        let factors: [Vec<f64>; 3] = Axis::ALL.map(|a| {
            let h = self.grid.step(a);
            self.kappa_halves[a.index()].iter().map(|k| k / h).collect()
        });
        let mut h = std::mem::take(&mut self.h);
        for component in Axis::ALL {
            let (a, b) = component.others();
            Self::curl_update_reference(
                self.grid,
                &mut h[component.index()],
                component,
                (&self.e[a.index()], &self.e[b.index()]),
                &offsets,
                &factors,
                None,
                self.dt,
            );
        }
        self.h = h;
        self.update_slabs_reference(Field::H, &offsets);
    }

    pub(crate) fn update_e_reference(&mut self) {
        let offsets = self.offsets(false);
        let factors: [Vec<f64>; 3] = Axis::ALL.map(|a| {
            let h = self.grid.step(a);
            self.kappa_nodes[a.index()].iter().map(|k| k / h).collect()
        });
        let mut e = std::mem::take(&mut self.e);
        for component in Axis::ALL {
            let (a, b) = component.others();
            let c = component.index();
            Self::curl_update_reference(
                self.grid,
                &mut e[c],
                component,
                (&self.h[a.index()], &self.h[b.index()]),
                &offsets,
                &factors,
                Some((&self.ca[c], &self.cb[c])),
                self.dt,
            );
        }
        self.e = e;
        self.update_slabs_reference(Field::E, &offsets);
    }

    /// The CPML's convolutions for `field`'s updates, added to the fields in their slabs.
    fn update_slabs_reference(&mut self, field: Field, offsets: &[Vec<Option<isize>>; 3]) {
        let grid = self.grid;
        let plane = grid.nx * grid.ny;
        let forward = field == Field::H;
        let mut slabs = std::mem::take(&mut self.slabs);
        for slab in slabs.iter_mut().filter(|s| s.field == field) {
            let (w, c) = (slab.axis, slab.component);
            let (first, second) = c.others();
            // in c's curl, the derivative along `first` (of F_second) enters with +1, along
            // `second` (of F_first) with −1
            let (sign, differentiated) = if w == first {
                (1.0, second)
            } else {
                (-1.0, first)
            };
            let h = grid.step(w);
            let source = match field {
                Field::E => &self.h[differentiated.index()],
                Field::H => &self.e[differentiated.index()],
            };
            // the slab's extent along each axis, and its planes of ψ
            let mut from = [0usize; 3];
            let mut count = [grid.nx, grid.ny, grid.nz];
            from[w.index()] = slab.from;
            count[w.index()] = slab.count;
            let local_plane = count[0] * count[1];
            let (bs, as_) = (&slab.b, &slab.a);
            let start = slab.from;
            let off = &offsets[w.index()];
            slab.psi
                .par_chunks_mut(local_plane)
                .enumerate()
                .for_each(|(lk, psi)| {
                    for lj in 0..count[1] {
                        for li in 0..count[0] {
                            let m = [from[0] + li, from[1] + lj, from[2] + lk];
                            let r = m[2] * plane + m[1] * grid.nx + m[0];
                            let mw = m[w.index()];
                            let other = off[mw].map_or(0.0, |o| source[(r as isize + o) as usize]);
                            let d = if forward {
                                other - source[r]
                            } else {
                                source[r] - other
                            } / h;
                            let q = lj * count[0] + li;
                            let s = mw - start;
                            psi[q] = bs[s] * psi[q] + as_[s] * d;
                        }
                    }
                });
            let cb = &self.cb[c.index()];
            let dt = self.dt;
            let psi = &slab.psi;
            let target = match field {
                Field::E => &mut self.e[c.index()],
                Field::H => &mut self.h[c.index()],
            };
            target
                .par_chunks_mut(plane)
                .enumerate()
                .skip(from[2])
                .take(count[2])
                .for_each(|(k, values)| {
                    let lk = k - from[2];
                    for lj in 0..count[1] {
                        for li in 0..count[0] {
                            let (i, j) = (from[0] + li, from[1] + lj);
                            let u = sign * psi[lk * local_plane + lj * count[0] + li];
                            let v = &mut values[j * grid.nx + i];
                            match field {
                                Field::E => *v += cb[k * plane + j * grid.nx + i] * u,
                                Field::H => *v -= dt * u,
                            }
                        }
                    }
                });
        }
        self.slabs = slabs;
    }
}

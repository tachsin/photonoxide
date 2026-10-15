//! Which of a cross-section's modes are guided, and which are the window's own.
//!
//! The window is closed: beyond its edges the field is zero, or a wall mirrors it. Below the
//! index of the media around the guides, the physical modes are leaky or radiating, and the
//! closed window can't hold them: there it has modes of its own instead, the standing waves of
//! the box its edges make, n_eff below the cladding's and even below 1. A count asked of
//! [`super::modes`] gets the guided modes first and then these. The rule that tells them apart
//! is a modes job's: a mode is guided when its n_eff is above the highest index of the media
//! around the guides, which a cross-section finds at its open edges
//! ([`CrossSection::surrounding_index`]).

use super::{Boundaries, Boundary, CrossSection, Permittivity, VectorMode};
use crate::Result;
use crate::units::Wavelength;

/// An edge of the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    West,
    East,
    South,
    North,
}

/// A cell's index, as the solver's shift takes it: from the larger real part of ε_xx and ε_yy.
fn index(e: &Permittivity) -> f64 {
    e.xx.re.max(e.yy.re).max(0.0).sqrt()
}

impl CrossSection {
    /// The highest index of the media around the cross-section's guides, at `wavelength`: a
    /// mode is guided when its n_eff is above it ([`VectorMode::is_guided`]), the rule a modes
    /// job keeps a count's modes by. Below it a mode isn't the guides': it is a mode of the
    /// closed box the window makes, its n_eff below the cladding's, even below 1.
    ///
    /// The media around the guides are those at the window's open edges, the ones with
    /// [`Boundary::Zero`]: a wall is a plane of symmetry through the guides, not their
    /// surroundings. Along each open edge, the highest index a wave can travel with:
    ///
    /// - the edge's media, where the highest of them lies at a corner with another open edge
    ///   (a substrate, a cladding, a layer's background): a strip's window, the substrate's
    ///   or the cladding's index;
    /// - where a guide runs into the edge, its index above the media at the edge's open ends
    ///   (a rib's slab), the fastest wave along the edge, the edge's own slab mode, solved on the
    ///   edge's cells drawn out across a mirror wall on each side (both kinds, the higher), by
    ///   the same scheme as [`super::modes`]: a mode below it leaks into the slab.
    ///
    /// A modes job reads the same media from its stack (the layers' backgrounds, and the
    /// substrate and cladding its window reaches), the same index unless a shape reaches the
    /// window's side: here, a guide the window's side cuts through runs into the edge, and its
    /// slab mode is what surrounds the guides, so keep the window wide of every guide but a
    /// slab.
    ///
    /// 0 when no edge is open: nothing lies around the guides, and every mode is guided. The
    /// window is taken as closed: a [`super::Pml`] doesn't change the index, and a leaky mode
    /// below it isn't guided (see [`super::modes`] with a PML for those). A bent cross-section's
    /// outer edge has the bend's conformal index, so none of its modes is guided: they are all
    /// leaky.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] if an edge's slab mode can't be solved.
    pub fn surrounding_index(&self, wavelength: Wavelength) -> Result<f64> {
        let b = self.boundaries;
        let mut highest = 0.0f64;
        for (edge, boundary, ends) in [
            (Edge::West, b.west, [b.south, b.north]),
            (Edge::East, b.east, [b.south, b.north]),
            (Edge::South, b.south, [b.west, b.east]),
            (Edge::North, b.north, [b.west, b.east]),
        ] {
            if boundary != Boundary::Zero {
                continue;
            }
            let cells = self.edge_cells(edge);
            let peak = cells.iter().map(index).fold(0.0, f64::max);
            // the media at the edge's ends that meet another open edge: a corner of the window
            let open = [cells[0], cells[cells.len() - 1]]
                .iter()
                .zip(ends)
                .filter(|&(_, end)| end == Boundary::Zero)
                .map(|(e, _)| index(e))
                .fold(0.0, f64::max);
            let uniform = cells.iter().all(|e| index(e) == peak);
            let along = if uniform || peak <= open {
                peak
            } else {
                // a guide runs into the edge: its slab mode, if it has one above the corners
                open.max(self.edge_slab(edge, &cells, wavelength)?)
            };
            highest = highest.max(along);
        }
        Ok(highest)
    }

    /// The cells along an edge, west to east or south to north.
    fn edge_cells(&self, edge: Edge) -> Vec<Permittivity> {
        let (nx, ny) = (self.x.len() - 1, self.y.len() - 1);
        match edge {
            Edge::West => (0..ny).map(|j| self.cells[j]).collect(),
            Edge::East => (0..ny).map(|j| self.cells[(nx - 1) * ny + j]).collect(),
            Edge::South => (0..nx).map(|i| self.cells[i * ny]).collect(),
            Edge::North => (0..nx).map(|i| self.cells[i * ny + ny - 1]).collect(),
        }
    }

    /// The highest Re n_eff of a wave along an edge: the edge's cells (their diagonal part)
    /// drawn out two cells across, between mirror walls of one kind, the edge's ends as the
    /// window's. A field uniform across is a mode of it, the edge's slab mode; of the two kinds
    /// of wall, one holds the TE-like slab mode and the other the TM-like, and the higher is
    /// taken.
    fn edge_slab(&self, edge: Edge, cells: &[Permittivity], wavelength: Wavelength) -> Result<f64> {
        let diagonal = |e: &Permittivity| Permittivity {
            xy: faer::c64::new(0.0, 0.0),
            yx: faer::c64::new(0.0, 0.0),
            ..*e
        };
        let (nx, ny) = (self.x.len() - 1, self.y.len() - 1);
        let b = self.boundaries;
        let mut highest = 0.0f64;
        for wall in [Boundary::ElectricWall, Boundary::MagneticWall] {
            let slab = match edge {
                Edge::West | Edge::East => {
                    let width = if edge == Edge::West {
                        self.x[1] - self.x[0]
                    } else {
                        self.x[nx] - self.x[nx - 1]
                    };
                    let cells = [cells, cells].concat().iter().map(diagonal).collect();
                    CrossSection::new(vec![0.0, width, 2.0 * width], self.y.clone(), cells)?
                        .with_boundaries(Boundaries {
                            west: wall,
                            east: wall,
                            south: b.south,
                            north: b.north,
                        })?
                }
                Edge::South | Edge::North => {
                    let height = if edge == Edge::South {
                        self.y[1] - self.y[0]
                    } else {
                        self.y[ny] - self.y[ny - 1]
                    };
                    // the library's cells are i * ny + j: each of the row's twice
                    let cells = cells.iter().flat_map(|e| [diagonal(e); 2]).collect();
                    CrossSection::new(self.x.clone(), vec![0.0, height, 2.0 * height], cells)?
                        .with_boundaries(Boundaries {
                            west: b.west,
                            east: b.east,
                            south: wall,
                            north: wall,
                        })?
                }
            };
            for mode in super::modes(&slab, wavelength, 1, None)? {
                highest = highest.max(mode.effective_index().re);
            }
        }
        Ok(highest)
    }
}

impl VectorMode {
    /// Whether the mode is guided: its n_eff above `surrounding_index`, the highest index of the
    /// media around the guides, as [`CrossSection::surrounding_index`] gives it for the
    /// cross-section the mode was solved on. A mode below it is the closed window's own.
    pub fn is_guided(&self, surrounding_index: f64) -> bool {
        self.effective_index().re > surrounding_index
    }
}

/// Of the `count` modes [`super::modes`] finds nearest `near`, the guided ones
/// ([`VectorMode::is_guided`] above [`CrossSection::surrounding_index`]), in the same order and
/// with the same bits: fewer than `count` when fewer are guided, none when none is. The count
/// is solved, then the window's own modes left out; it doesn't solve further to make the count
/// up. For every guided mode, however many, [`super::modes_above`] with the surrounding index
/// as its threshold.
///
/// # Errors
///
/// As [`super::modes`] and [`CrossSection::surrounding_index`].
pub fn guided_modes(
    cs: &CrossSection,
    wavelength: Wavelength,
    count: usize,
    near: Option<f64>,
) -> Result<Vec<VectorMode>> {
    // (never asked to stop, it always has its modes or an error)
    Ok(guided_modes_until(cs, wavelength, count, near, || false)?.unwrap_or_default())
}

/// As [`guided_modes`], giving up when `stop` says so, as [`super::modes_until`] does: `None`
/// then.
///
/// # Errors
///
/// As [`guided_modes`].
pub fn guided_modes_until(
    cs: &CrossSection,
    wavelength: Wavelength,
    count: usize,
    near: Option<f64>,
    stop: impl Fn() -> bool,
) -> Result<Option<Vec<VectorMode>>> {
    let Some(modes) = super::modes_until(cs, wavelength, count, near, stop)? else {
        return Ok(None);
    };
    let surrounding = cs.surrounding_index(wavelength)?;
    Ok(Some(
        modes
            .into_iter()
            .filter(|m| m.is_guided(surrounding))
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::super::{modes, strip};
    use super::*;
    use crate::mode::Polarization;
    use crate::mode::marcatili::{Family, Rectangle};
    use crate::mode::multilayer::Multilayer;
    use crate::units::Length;
    use faer::c64;

    fn iso(n: f64) -> Permittivity {
        Permittivity::isotropic(c64::new(n * n, 0.0))
    }

    fn lam() -> Wavelength {
        Wavelength::um(1.55).unwrap()
    }

    /// A rectangle of index 1.6, 3 × 1.2 µm, in 1.45, on a 50 nm grid with its edges on nodes,
    /// 1 µm of cladding all round.
    fn rectangle() -> CrossSection {
        CrossSection::uniform((-2.5, 2.5, 100), (-1.6, 1.6, 64), |x, y| {
            iso(if x.abs() < 1.5 && y.abs() < 0.6 {
                1.6
            } else {
                1.45
            })
        })
        .unwrap()
    }

    #[test]
    fn a_rectangle_keeps_exactly_the_guided_modes_marcatili_counts() {
        let l = lam();
        // Marcatili's modes, E^x_pq and E^y_pq with k_z above the cladding's: four, the nearest
        // to cut-off 0.04 above it, the next pair beyond it (p = 3 or q = 2)
        let r = Rectangle::uniform(1.6, 1.45, Length::um(3.0), Length::um(1.2)).unwrap();
        let mut marcatili: Vec<f64> = [Family::Ex, Family::Ey]
            .into_iter()
            .flat_map(|f| (1..=6).flat_map(move |p| (1..=6).map(move |q| (f, p, q))))
            .filter_map(|(f, p, q)| r.mode(f, p, q, l))
            .collect();
        marcatili.sort_by(|a, b| b.total_cmp(a));
        assert_eq!(marcatili.len(), 4, "{marcatili:?}");
        assert!(marcatili[3] > 1.48, "{marcatili:?}");

        let cs = rectangle();
        let cladding = (1.45f64 * 1.45).sqrt();
        assert_eq!(cs.surrounding_index(l).unwrap(), cladding);
        // the raw call still gives all twenty: the four guided ones, then the window's own
        let all = modes(&cs, l, 20, None).unwrap();
        assert_eq!(all.len(), 20);
        let guided = guided_modes(&cs, l, 20, None).unwrap();
        assert_eq!(guided.len(), 4);
        // the guided ones are the raw call's first four, to the bit, and Marcatili's
        for (k, (g, a)) in guided.iter().zip(&all).enumerate() {
            assert_eq!(g, a);
            assert!(g.is_guided(cladding));
            let n = g.effective_index().re;
            assert!((n - marcatili[k]).abs() < 1e-2, "{n} {}", marcatili[k]);
        }
        // what is dropped is the window's own: every one below the cladding
        for m in &all[4..] {
            assert!(!m.is_guided(cladding));
            assert!(m.effective_index().re < 1.45, "{}", m.effective_index());
        }
        // a count within the guided ones is the raw call's, whole
        let two = guided_modes(&cs, l, 2, None).unwrap();
        assert_eq!(two, modes(&cs, l, 2, None).unwrap());
    }

    #[test]
    fn a_ribs_slab_at_the_windows_edge_is_what_surrounds_it() {
        let l = lam();
        // a 500 nm silicon rib 220 nm tall on a 100 nm slab that runs to both sides, oxide
        // below and air above, on a 20 nm grid with every interface on nodes
        let cs = CrossSection::uniform((-1.51, 1.51, 151), (-0.8, 0.8, 80), |x, y| {
            let si = (0.0..0.1).contains(&y) || ((0.0..0.22).contains(&y) && x.abs() < 0.25);
            iso(if si {
                3.48
            } else if y < 0.0 {
                1.444
            } else {
                1.0
            })
        })
        .unwrap();
        let surrounding = cs.surrounding_index(l).unwrap();
        // the slab's TE mode, exactly: the solver's on the edge within its grid's error
        let slab = Multilayer::new(
            c64::new(1.0, 0.0),
            &[(c64::new(3.48, 0.0), Length::um(0.1))],
            c64::new(1.444, 0.0),
        )
        .unwrap()
        .bound_modes(Polarization::Te, l)
        .unwrap()[0]
            .effective_index()
            .re;
        assert!((surrounding - slab).abs() < 2e-2, "{surrounding} {slab}");
        let all = modes(&cs, l, 8, None).unwrap();
        let guided = guided_modes(&cs, l, 8, None).unwrap();
        // the rib's fundamental is guided, above the slab; the slab's own modes, which the
        // window's sides make standing waves of, are left out though they are above the oxide
        assert!(!guided.is_empty());
        assert_eq!(guided[0], all[0]);
        assert!(guided.iter().all(|m| m.effective_index().re > surrounding));
        let dropped: Vec<f64> = all
            .iter()
            .map(|m| m.effective_index().re)
            .filter(|&n| n <= surrounding)
            .collect();
        assert_eq!(guided.len() + dropped.len(), all.len());
        assert!(dropped.iter().any(|&n| n > 1.444), "{dropped:?}");
    }

    #[test]
    fn walls_are_planes_of_symmetry_not_surroundings() {
        let l = lam();
        // a quarter of a silicon strip in oxide: the core on the west and south edges
        let quarter = || {
            CrossSection::uniform((0.0, 1.05, 21), (0.0, 0.75, 15), |x, y| {
                iso(if x < 0.25 && y < 0.11 { 3.473 } else { 1.444 })
            })
            .unwrap()
        };
        let oxide = (1.444f64 * 1.444).sqrt();
        // with mirror walls there the oxide surrounds it
        let walled = quarter()
            .with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                south: Boundary::MagneticWall,
                ..Boundaries::default()
            })
            .unwrap();
        assert_eq!(walled.surrounding_index(l).unwrap(), oxide);
        let fundamental = &guided_modes(&walled, l, 1, None).unwrap()[0];
        assert!(fundamental.is_guided(oxide) && fundamental.te_fraction() > 0.9);
        // without them the core reaches the window's corner, open on both sides: nothing above
        // it, so nothing is guided
        let open = quarter();
        assert_eq!(
            open.surrounding_index(l).unwrap(),
            (3.473f64 * 3.473).sqrt()
        );
        assert!(guided_modes(&open, l, 2, None).unwrap().is_empty());
        // walls all round: nothing surrounds it
        let closed = quarter()
            .with_boundaries(Boundaries {
                west: Boundary::ElectricWall,
                east: Boundary::ElectricWall,
                south: Boundary::MagneticWall,
                north: Boundary::MagneticWall,
            })
            .unwrap();
        assert_eq!(closed.surrounding_index(l).unwrap(), 0.0);
        // the book's whole strip: the oxide
        assert_eq!(strip(0.05).surrounding_index(l).unwrap(), oxide);
    }

    #[test]
    fn a_stopped_guided_solve_gives_nothing() {
        let cs = strip(0.05);
        assert!(
            guided_modes_until(&cs, lam(), 2, None, || true)
                .unwrap()
                .is_none()
        );
        assert!(guided_modes(&cs, lam(), 0, None).is_err());
    }
}

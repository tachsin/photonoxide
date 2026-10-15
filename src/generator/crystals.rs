//! Photonic crystals: a slab less a square or hexagonal lattice of holes, with an L3 or H1
//! cavity or a W1 waveguide; and the inverse-taper edge coupler.

use super::waveguides::{Taper, TaperProfile, reason, strip};
use super::{Drawing, Generator, Port, positive, refuse, um};
use crate::Result;
use crate::geometry::{Array, Lattice, Point, Shape};
use crate::units::Length;

/// A photonic crystal's lattice of holes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CrystalLattice {
    /// Holes at (i a, j a).
    Square,
    /// Holes in rows along x, j a √3/2 apart, the odd rows half a pitch along (Γ–K along x).
    Hexagonal,
}

/// A photonic crystal's defect.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Defect {
    /// None: the bare lattice.
    None,
    /// Three holes missing in a row, (−1, 0) to (1, 0), and the holes at its ends, (∓2, 0),
    /// moved out along the row by `shift` (Akahane et al. 2003: 0.15 a).
    L3 {
        /// How far each end hole moves out.
        shift: Length,
    },
    /// One hole missing, (0, 0).
    H1,
    /// The row j = 0 missing: a line defect, a guide along x.
    W1,
}

/// A photonic-crystal slab: a rectangle of the layer, less a square or hexagonal lattice of
/// holes of `radius`, `pitch` apart, sites i from −`half[0]` to `half[0]` and j from
/// −`half[1]` to `half[1]`, with a defect. The rectangle reaches half a pitch beyond the outer
/// holes' centres (and the odd rows' half-pitch shift).
///
/// A W1 guide has ports `o1` and `o2` where its row meets the rectangle's ends, facing 180°
/// and 0°, as wide as the slab between the holes of the rows beside it: √3 a − 2r (hexagonal),
/// 2a − 2r (square). The cavities have none.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct PhotonicCrystal {
    /// The lattice.
    pub lattice: CrystalLattice,
    /// The pitch, a.
    pub pitch: Length,
    /// The holes' radius, r.
    pub radius: Length,
    /// The sites either side of the centre: i from −`half[0]` to `half[0]`, j from −`half[1]`
    /// to `half[1]`.
    pub half: [i64; 2],
    /// The defect.
    pub defect: Defect,
}

impl PhotonicCrystal {
    /// A hexagonal lattice of holes with `defect`.
    pub fn hexagonal(
        pitch: Length,
        radius: Length,
        half: [i64; 2],
        defect: Defect,
    ) -> PhotonicCrystal {
        PhotonicCrystal {
            lattice: CrystalLattice::Hexagonal,
            pitch,
            radius,
            half,
            defect,
        }
    }

    /// The lattice, its defect made.
    ///
    /// # Errors
    ///
    /// [`crate::Error::InvalidValue`] for a pitch that isn't positive, or a defect at a site
    /// the lattice doesn't have.
    pub fn sites(&self) -> Result<Lattice> {
        let k = "photonic crystal";
        let a = self.pitch;
        let [ni, nj] = self.half;
        let (i, j) = ([-ni, ni], [-nj, nj]);
        let mut l = match self.lattice {
            CrystalLattice::Square => Lattice::rectangular([a, a], i, j)?,
            CrystalLattice::Hexagonal => Lattice::hexagonal(a, i, j)?,
        };
        let err = |e| refuse(k, reason(e));
        match self.defect {
            Defect::None => {}
            Defect::H1 => l = l.remove([0, 0]).map_err(err)?,
            Defect::L3 { shift } => {
                for s in [-1, 0, 1] {
                    l = l.remove([s, 0]).map_err(err)?;
                }
                l = l
                    .shift([-2, 0], [-shift, Length::ZERO])
                    .map_err(err)?
                    .shift([2, 0], [shift, Length::ZERO])
                    .map_err(err)?;
            }
            Defect::W1 => {
                for s in -ni..=ni {
                    l = l.remove([s, 0]).map_err(err)?;
                }
            }
        }
        Ok(l)
    }

    /// The slab's extent: [x₀, x₁] and [y₀, y₁], µm.
    fn extent(&self) -> ([f64; 2], [f64; 2]) {
        let a = self.pitch.to_um();
        let [ni, nj] = self.half;
        let (ni, nj) = (ni as f64, nj as f64);
        let row = match self.lattice {
            CrystalLattice::Square => a,
            CrystalLattice::Hexagonal => a * 3f64.sqrt() / 2.0,
        };
        let odd = match self.lattice {
            CrystalLattice::Square => 0.0,
            CrystalLattice::Hexagonal => a / 2.0,
        };
        (
            [-ni * a - a / 2.0, ni * a + odd + a / 2.0],
            [-nj * row - a / 2.0, nj * row + a / 2.0],
        )
    }
}

impl Generator for PhotonicCrystal {
    fn kind(&self) -> &str {
        "photonic crystal"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let a = positive(k, "pitch", self.pitch, false)?;
        let r = positive(k, "radius", self.radius, false)?;
        if r >= a / 2.0 {
            return Err(refuse(
                k,
                format!("its holes' radius {r} um must be less than half its pitch {a} um"),
            ));
        }
        if self.half[0] < 2 || self.half[1] < 1 {
            return Err(refuse(
                k,
                format!(
                    "it needs at least 2 sites either side along x and 1 along y, got {:?}",
                    self.half
                ),
            ));
        }
        let lattice = self.sites()?;
        let holes = Array::of(&lattice, &Shape::circle(Point::um(0.0, 0.0), um(r))?)
            .map_err(|e| refuse(k, reason(e)))?;
        let ([x0, x1], [y0, y1]) = self.extent();
        let slab = Shape::rect(
            Point::um((x0 + x1) / 2.0, (y0 + y1) / 2.0),
            um(x1 - x0),
            um(y1 - y0),
        )?;
        let mut d = Drawing::new(k);
        d.add(
            layer,
            slab.difference(&Shape::Array(holes))
                .map_err(|e| refuse(k, reason(e)))?,
        );
        if self.defect == Defect::W1 {
            let width = match self.lattice {
                CrystalLattice::Square => 2.0 * a - 2.0 * r,
                CrystalLattice::Hexagonal => 3f64.sqrt() * a - 2.0 * r,
            };
            d.add_port(Port::new("o1", Point::um(x0, 0.0), 180.0, um(width), layer))?;
            d.add_port(Port::new("o2", Point::um(x1, 0.0), 0.0, um(width), layer))?;
        }
        Ok(d)
    }
}

/// An inverse-taper edge coupler: a tip `tip_width` wide at the chip's facet (x = 0), tapering
/// along x over `taper_length` to the guide's `width`, then a straight `straight` long.
///
/// Ports `o1` at the tip (0, 0) facing 180°, `tip_width` wide (the facet, where a fibre's mode
/// meets the tip's spread one), and `o2` (L_t + L_s, 0) facing 0°, `width` wide.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct EdgeCoupler {
    /// The tip's width at the facet.
    pub tip_width: Length,
    /// The guide's width.
    pub width: Length,
    /// The taper's length.
    pub taper_length: Length,
    /// The straight after it (zero: none).
    pub straight: Length,
    /// How the width goes along the taper.
    pub profile: TaperProfile,
}

impl EdgeCoupler {
    /// A linear inverse taper from `tip_width` to `width` over `taper_length`.
    pub fn new(tip_width: Length, width: Length, taper_length: Length) -> EdgeCoupler {
        EdgeCoupler {
            tip_width,
            width,
            taper_length,
            straight: Length::ZERO,
            profile: TaperProfile::Linear,
        }
    }
}

impl Generator for EdgeCoupler {
    fn kind(&self) -> &str {
        "edge coupler"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let tip = positive(k, "tip_width", self.tip_width, false)?;
        let w = positive(k, "width", self.width, false)?;
        let lt = positive(k, "taper_length", self.taper_length, false)?;
        let ls = positive(k, "straight", self.straight, true)?;
        if tip >= w {
            return Err(refuse(
                k,
                format!("its tip {tip} um must be narrower than its guide {w} um"),
            ));
        }
        let taper = Taper::new(um(tip), um(w), um(lt))
            .with_profile(self.profile.clone())
            .draw(layer)
            .map_err(|e| refuse(k, reason(e)))?;
        let mut d = Drawing::new(k);
        for item in taper.items() {
            d.add_etched(&item.layer, item.shape.clone(), item.etch);
        }
        if ls > 0.0 {
            d.add(layer, strip(w, 0.0, lt, lt + ls)?);
        }
        d.add_port(Port::new("o1", Point::um(0.0, 0.0), 180.0, um(tip), layer))?;
        d.add_port(Port::new("o2", Point::um(lt + ls, 0.0), 0.0, um(w), layer))?;
        Ok(d)
    }
}

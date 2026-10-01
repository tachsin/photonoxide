//! The effective index method: a ridge waveguide's mode from two slab problems.
//!
//! L. Chrostowski, M. Hochberg, *Silicon Photonics Design*, Cambridge University Press (2015),
//! [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168), Section 3.2.5,
//! after G. B. Hocker, W. K. Burns, Appl. Opt. 16, 113 (1977),
//! [doi:10.1364/AO.16.000113](https://doi.org/10.1364/AO.16.000113):
//!
//! 1. the vertical slab through the ridge (cladding below, core, cladding above), and beside it
//!    (a rib's thinner slab, or a strip's cladding alone), give an effective index each;
//! 2. the lateral slab, the ridge's index between the sides', gives the mode's.
//!
//! For a TE-like mode (E mostly horizontal) the vertical slabs are solved as TE and the lateral
//! one as TM, since the field that lies along the vertical slab's layers lies across the
//! lateral slab's; a TM-like mode the reverse. The method assumes the field separates,
//! E(x, y) = E(x) E(y), which corners break: for the book's 500 × 220 nm strip it gives 2.489,
//! against 2.443 from 2D (Fig. 3.14), 1.9 % high; the book states 1.2 % for n_eff and 2.7 % for
//! the group index. Against photonoxide's full-vector solver (6.25 × 5 nm grid), 220 nm strips
//! of 3.473 in 1.444 at 1550 nm, TE-like: n_eff +3.9, +1.8 and +1.0 % and n_g −6.7, −3.7 and
//! −2.2 % at 400, 500 and 600 nm wide (the `effective_index_method` example). The narrower the
//! strip, the more of its light is near the corners.

use super::Polarization;
use super::slab::Slab;
use crate::units::{Length, Wavelength};
use crate::{Error, Result};

/// A ridge waveguide: a core `height` thick and `width` wide on a slab of the same material
/// `slab` thick (zero for a strip), between the cladding `below` and the cladding `above`
/// (which also fills the sides).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ridge {
    below: f64,
    core: f64,
    above: f64,
    height: Length,
    slab: Length,
    width: Length,
}

/// The effective index method's result.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Eim {
    /// The vertical slab's effective index through the ridge.
    pub centre: f64,
    /// Beside the ridge: the rib's slab mode, or a strip's upper cladding.
    pub side: f64,
    /// The mode's effective index, from the lateral slab.
    pub effective_index: f64,
}

impl Ridge {
    /// A ridge; `slab` is zero for a strip.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] for indices that aren't finite and at least 1, a core that isn't
    /// the highest, a height or width that isn't positive, or a slab that isn't thinner than
    /// the ridge.
    pub fn new(
        below: f64,
        core: f64,
        above: f64,
        height: Length,
        slab: Length,
        width: Length,
    ) -> Result<Ridge> {
        let ok = |n: f64| n.is_finite() && n >= 1.0;
        if !ok(below) || !ok(core) || !ok(above) || core <= below || core <= above {
            return Err(Error::invalid(
                "ridge",
                format!("the core ({core}) must have the highest index, not {below} and {above}"),
            ));
        }
        let positive = |l: Length| l.to_um().is_finite() && l.to_um() > 0.0;
        if !positive(height) || !positive(width) {
            return Err(Error::invalid(
                "ridge",
                format!("the height and width must be positive, got {height} and {width}"),
            ));
        }
        let s = slab.to_um();
        if !(s.is_finite() && s >= 0.0 && s < height.to_um()) {
            return Err(Error::invalid(
                "ridge",
                format!("the slab must be thinner than the ridge, got {slab} under {height}"),
            ));
        }
        Ok(Ridge {
            below,
            core,
            above,
            height,
            slab,
            width,
        })
    }

    /// A strip: a ridge with no slab.
    ///
    /// # Errors
    ///
    /// As [`Ridge::new`].
    pub fn strip(
        below: f64,
        core: f64,
        above: f64,
        height: Length,
        width: Length,
    ) -> Result<Ridge> {
        Ridge::new(below, core, above, height, Length::ZERO, width)
    }

    /// The fundamental TE-like (`Polarization::Te`) or TM-like (`Polarization::Tm`) mode at
    /// `wavelength`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if a slab on the way guides no mode: the ridge's vertical slab,
    /// a rib's side slab, or the lateral slab (the ridge's index no higher than the sides').
    pub fn mode(&self, like: Polarization, wavelength: Wavelength) -> Result<Eim> {
        let (vertical, lateral) = match like {
            Polarization::Te => (Polarization::Te, Polarization::Tm),
            Polarization::Tm => (Polarization::Tm, Polarization::Te),
        };
        let fundamental = |slab: Slab, polarization: Polarization, what: &str| {
            slab.modes(polarization, wavelength)
                .first()
                .map(|m| m.effective_index())
                .ok_or_else(|| {
                    Error::invalid("effective index method", format!("{what} guides no mode"))
                })
        };
        let centre = fundamental(
            Slab::new(self.below, self.core, self.above, self.height)?,
            vertical,
            "the ridge's vertical slab",
        )?;
        let side = if self.slab.to_um() > 0.0 {
            fundamental(
                Slab::new(self.below, self.core, self.above, self.slab)?,
                vertical,
                "the rib's side slab",
            )?
        } else {
            self.above
        };
        if centre <= side {
            return Err(Error::invalid(
                "effective index method",
                format!(
                    "the ridge ({centre}) is no higher than its sides ({side}): no lateral mode"
                ),
            ));
        }
        let effective_index = fundamental(
            Slab::new(side, centre, side, self.width)?,
            lateral,
            "the lateral slab",
        )?;
        Ok(Eim {
            centre,
            side,
            effective_index,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lam(um: f64) -> Wavelength {
        Wavelength::um(um).unwrap()
    }

    #[test]
    fn the_books_strip_gives_its_effective_index() {
        // Section 3.2.5 and Listing 3.10: slab TE 2.845, then the lateral slab with that index
        // as its core, solved by Lumerical's 1D mode solver on a 10 nm mesh: 2.489. On the
        // book's own input the exact lateral TM slab gives 2.488558, which rounds to it; from
        // our unrounded 2.844816 the method gives 2.488368, the 6.3e-4 being the rounded input
        // (1.9e-4) and the book's mesh
        let lateral = Slab::new(1.444, 2.845, 1.444, Length::nm(500.0)).unwrap();
        let n = lateral.modes(Polarization::Tm, lam(1.55))[0].effective_index();
        assert!((n - 2.489).abs() < 5e-4, "{n}");
        let strip =
            Ridge::strip(1.444, 3.473, 1.444, Length::nm(220.0), Length::nm(500.0)).unwrap();
        let eim = strip.mode(Polarization::Te, lam(1.55)).unwrap();
        assert!((eim.centre - 2.845).abs() < 5e-4, "{}", eim.centre);
        assert_eq!(eim.side, 1.444);
        assert!(
            (eim.effective_index - 2.489).abs() < 1e-3,
            "{}",
            eim.effective_index
        );
    }

    #[test]
    fn a_wide_ridge_tends_to_its_slab() {
        // the lateral slab's index approaches its core's as it widens
        for like in [Polarization::Te, Polarization::Tm] {
            let wide = |w: f64| {
                Ridge::strip(1.444, 3.473, 1.444, Length::nm(220.0), Length::um(w))
                    .unwrap()
                    .mode(like, lam(1.55))
                    .unwrap()
            };
            let (a, b) = (wide(5.0), wide(50.0));
            assert!(a.effective_index < b.effective_index && b.effective_index < b.centre);
            assert!(b.centre - b.effective_index < 1e-4, "{like:?}: {b:?}");
        }
    }

    #[test]
    fn a_ribs_sides_are_its_slab() {
        // the book's rib: 220 nm with a 90 nm slab, 500 nm wide; the sides guide too
        let rib = Ridge::new(
            1.444,
            3.473,
            1.444,
            Length::nm(220.0),
            Length::nm(90.0),
            Length::nm(500.0),
        )
        .unwrap();
        let eim = rib.mode(Polarization::Te, lam(1.55)).unwrap();
        let side = Slab::new(1.444, 3.473, 1.444, Length::nm(90.0))
            .unwrap()
            .modes(Polarization::Te, lam(1.55))[0]
            .effective_index();
        assert_eq!(eim.side, side);
        let strip =
            Ridge::strip(1.444, 3.473, 1.444, Length::nm(220.0), Length::nm(500.0)).unwrap();
        // the slab beside it pulls the mode outwards, and its index up
        assert!(
            eim.effective_index
                > strip
                    .mode(Polarization::Te, lam(1.55))
                    .unwrap()
                    .effective_index
        );
    }

    #[test]
    fn bad_ridges_are_errors() {
        let (h, w) = (Length::nm(220.0), Length::nm(500.0));
        assert!(Ridge::strip(1.444, 1.4, 1.444, h, w).is_err());
        assert!(Ridge::strip(1.444, 3.473, 1.444, Length::ZERO, w).is_err());
        assert!(Ridge::new(1.444, 3.473, 1.444, h, Length::nm(220.0), w).is_err());
        // a strip too thin to guide at 1.55 µm between unequal claddings
        let thin = Ridge::strip(1.444, 1.6, 1.0, Length::nm(20.0), w).unwrap();
        assert!(thin.mode(Polarization::Te, lam(1.55)).is_err());
    }
}

//! How a shape's edges are etched ([`Etch`]), which [`crate::geometry`]'s etched bodies and
//! [`crate::stack`]'s structures both read. It is [`crate::stack::Etch`] there; it lives here, under
//! both, so that the geometry doesn't import the stack.

use crate::units::Length;

/// How the edges of a shape on a layer were etched: how deep from the layer's top, and the
/// angle the walls make with the layer's plane.
///
/// Build one with [`Etch::FULL`], [`Etch::partial`] and [`Etch::with_sidewall`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Etch {
    /// How deep the etch went from the layer's top: `None` through the whole layer (a strip),
    /// or a depth less than the layer's thickness (a rib: the layer's material is left below
    /// it, everywhere).
    pub depth: Option<Length>,
    /// The walls' angle from the layer's plane, radians, above 0 and at most π/2: π/2 for
    /// vertical walls, less for walls that lean out going down, so that a strip's
    /// cross-section is a trapezoid wider at its foot. The shape's outline is its top face.
    pub sidewall: f64,
}

impl Default for Etch {
    fn default() -> Etch {
        Etch::FULL
    }
}

impl Etch {
    /// Through the whole layer, with vertical walls: what [`crate::stack::Structure::draw`] draws.
    pub const FULL: Etch = Etch {
        depth: None,
        sidewall: std::f64::consts::FRAC_PI_2,
    };

    /// An etch `depth` deep from the layer's top, with vertical walls.
    pub fn partial(depth: Length) -> Etch {
        Etch {
            depth: Some(depth),
            ..Etch::FULL
        }
    }

    /// The same etch with walls at `radians` from the layer's plane.
    #[must_use]
    pub fn with_sidewall(self, radians: f64) -> Etch {
        Etch {
            sidewall: radians,
            ..self
        }
    }

    /// The same etch with walls at `degrees` from the layer's plane: 90 exactly is
    /// [`Etch::FULL`]'s π/2.
    #[must_use]
    pub fn with_sidewall_deg(self, degrees: f64) -> Etch {
        self.with_sidewall(if degrees == 90.0 {
            std::f64::consts::FRAC_PI_2
        } else {
            degrees.to_radians()
        })
    }

    /// How far a wall reaches out per unit of depth, cot θ: 0 for a vertical wall (θ within
    /// 1e-12 of π/2).
    pub fn run(&self) -> f64 {
        let (s, c) = self.sidewall.sin_cos();
        if c.abs() <= 1e-12 { 0.0 } else { c / s }
    }

    /// Whether the etch goes through the layer with vertical walls, as [`Etch::FULL`].
    pub fn is_full(&self) -> bool {
        self.depth.is_none() && self.run() == 0.0
    }
}

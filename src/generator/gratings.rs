//! Gratings: a Bragg grating of sidewall corrugations (uniform, apodized or sampled), a
//! subwavelength-grating waveguide, and a grating coupler (uniform or apodized).

use super::waveguides::{reason, strip};
use super::{Drawing, Generator, Port, positive, refuse, um};
use crate::Result;
use crate::expr::{Expr, Parameters};
use crate::geometry::{Array, Element, Point, Polygon, Shape};
use crate::stack::Etch;
use crate::units::Length;

/// `e` at x = `x` (µm) with `p`'s parameters, as a value in its unit (a length in µm).
fn at_x(e: &Expr, p: &Parameters, x: f64, kind: &str, what: &str) -> Result<f64> {
    let q = p.at_coordinate("x", x).map_err(|e| refuse(kind, e))?;
    e.try_eval(q.values())
        .map_err(|err| refuse(kind, format!("its {what} at x = {x} um: {err}")))
}

/// A Bragg grating of sidewall corrugations along x from the origin: a core strip of the
/// narrow width, and on each side a tooth spanning `duty` of every period, in its middle (so
/// the grating ends on its core at both ends, and is its own mirror image about L/2).
///
/// Recessed (as Wang et al. 2010 drew theirs): the teeth reach the full `width` w, and the guide
/// between them is w − 2Δw, Δw the `corrugation` on each side. Centred: the guide alternates
/// between w + Δw and w − Δw about its width. Apodized: the teeth's height is Δw(x), an
/// expression of the tooth's centre x (from the grating's start), the core still w − 2Δw
/// (recessed) or w − Δw (centred), so a tooth of zero height is left out. Sampled: teeth only
/// in bursts of `sampling[0]` periods every `sampling[1]` (a sampled grating's Z₁ and Z₀).
///
/// Ports `o1` (0, 0) facing 180° and `o2` (N Λ, 0) facing 0°, as wide as the core.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BraggGrating {
    /// The guide's width, w: the teeth's reach (recessed), or the mean width (centred).
    pub width: Length,
    /// The corrugation on each side, Δw (the largest, apodized).
    pub corrugation: Length,
    /// The period, Λ.
    pub period: Length,
    /// The fraction of each period a tooth spans, from 0 to 1.
    pub duty: f64,
    /// How many periods, N.
    pub periods: usize,
    /// Whether the corrugation is centred on the width rather than recessed into it.
    pub centred: bool,
    /// The teeth's height Δw(x), an expression of the tooth's centre `x` and of the parameters
    /// given with it, at most the corrugation; `None`: uniform.
    pub apodization: Option<(Expr, Parameters)>,
    /// Bursts of `[n₁, n₀]`: teeth in the first n₁ periods of every n₀; `None`: every period.
    pub sampling: Option<[usize; 2]>,
}

impl BraggGrating {
    /// A uniform recessed grating: `periods` periods of `period`, half of each a tooth.
    pub fn new(width: Length, corrugation: Length, period: Length, periods: usize) -> BraggGrating {
        BraggGrating {
            width,
            corrugation,
            period,
            duty: 0.5,
            periods,
            centred: false,
            apodization: None,
            sampling: None,
        }
    }

    /// The same grating, its teeth's height `apodization`, Δw(x).
    #[must_use]
    pub fn with_apodization(mut self, apodization: Expr, parameters: Parameters) -> BraggGrating {
        self.apodization = Some((apodization, parameters));
        self
    }

    /// The same grating, sampled: teeth in the first `burst` periods of every `every`.
    #[must_use]
    pub fn sampled(mut self, burst: usize, every: usize) -> BraggGrating {
        self.sampling = Some([burst, every]);
        self
    }

    /// Its length, N Λ.
    pub fn length(&self) -> Length {
        self.period * self.periods as f64
    }

    /// The core's width: w − 2Δw recessed, w − Δw centred.
    pub fn core_width(&self) -> Length {
        if self.centred {
            self.width - self.corrugation
        } else {
            self.width - self.corrugation * 2.0
        }
    }
}

impl Generator for BraggGrating {
    fn kind(&self) -> &str {
        "bragg grating"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let dw = positive(k, "corrugation", self.corrugation, false)?;
        let a = positive(k, "period", self.period, false)?;
        if !(self.duty > 0.0 && self.duty < 1.0) {
            return Err(refuse(
                k,
                format!("its duty must be between 0 and 1, got {}", self.duty),
            ));
        }
        if self.periods == 0 {
            return Err(refuse(k, "it needs at least one period"));
        }
        let core = self.core_width().to_um();
        if core <= 0.0 {
            return Err(refuse(
                k,
                format!("its corrugation {dw} um leaves no guide of its width {w} um"),
            ));
        }
        if let Some([burst, every]) = self.sampling
            && !(burst > 0 && burst <= every)
        {
            return Err(refuse(
                k,
                format!("its sampling needs bursts of 1 to {every} periods, got {burst}"),
            ));
        }
        let l = a * self.periods as f64;
        let tooth = self.duty * a;
        let mut d = Drawing::new(k);
        d.add(layer, strip(core, 0.0, 0.0, l)?);
        let mut teeth = Vec::with_capacity(2 * self.periods);
        for i in 0..self.periods {
            if let Some([burst, every]) = self.sampling
                && i % every >= burst
            {
                continue;
            }
            let xc = (i as f64 + 0.5) * a;
            let h = match &self.apodization {
                None => dw,
                Some((e, p)) => {
                    let h = at_x(e, p, xc, k, "apodization")?;
                    if !(h.is_finite() && (0.0..=dw * (1.0 + 1e-12)).contains(&h)) {
                        return Err(refuse(
                            k,
                            format!(
                                "its apodization gives {h} um at x = {} um: a tooth's height is \
                                 from 0 to the corrugation, {dw} um",
                                xc
                            ),
                        ));
                    }
                    h
                }
            };
            if h == 0.0 {
                continue;
            }
            let y = core / 2.0 + h / 2.0;
            for (j, side) in [(-1, -1.0), (1, 1.0)] {
                teeth.push(Element::new(
                    [i as i64, j],
                    Shape::rect(Point::um(xc, side * y), um(tooth), um(h))?,
                ));
            }
        }
        if !teeth.is_empty() {
            d.add(
                layer,
                Shape::Array(Array::new(teeth).map_err(|e| refuse(k, reason(e)))?),
            );
        }
        d.add_port(Port::new("o1", Point::um(0.0, 0.0), 180.0, um(core), layer))?;
        d.add_port(Port::new("o2", Point::um(l, 0.0), 0.0, um(core), layer))?;
        Ok(d)
    }
}

/// A subwavelength-grating waveguide along x from the origin: segments the first `duty` of each
/// `period` long, their widths going linearly from `width` to `width_end` (a taper), on a
/// continuous `bridge` along the middle if asked.
///
/// Ports `o1` (0, 0) facing 180°, `width` wide, and `o2` at the last segment's end,
/// ((N − 1 + duty) Λ, 0), facing 0°, `width_end` wide.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SwgWaveguide {
    /// The first segment's width.
    pub width: Length,
    /// The last segment's width.
    pub width_end: Length,
    /// The period, Λ.
    pub period: Length,
    /// The fraction of each period a segment spans, from 0 to 1.
    pub duty: f64,
    /// How many segments, N.
    pub periods: usize,
    /// A strip along the middle joining the segments (zero: none).
    pub bridge: Length,
}

impl SwgWaveguide {
    /// `periods` segments of `width`, `duty` of each `period` long.
    pub fn new(width: Length, period: Length, duty: f64, periods: usize) -> SwgWaveguide {
        SwgWaveguide {
            width,
            width_end: width,
            period,
            duty,
            periods,
            bridge: Length::ZERO,
        }
    }
}

impl Generator for SwgWaveguide {
    fn kind(&self) -> &str {
        "subwavelength grating"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w0 = positive(k, "width", self.width, false)?;
        let w1 = positive(k, "width_end", self.width_end, false)?;
        let a = positive(k, "period", self.period, false)?;
        let bridge = positive(k, "bridge", self.bridge, true)?;
        if !(self.duty > 0.0 && self.duty < 1.0) {
            return Err(refuse(
                k,
                format!("its duty must be between 0 and 1, got {}", self.duty),
            ));
        }
        if self.periods == 0 {
            return Err(refuse(k, "it needs at least one segment"));
        }
        if bridge >= w0.min(w1) {
            return Err(refuse(
                k,
                format!("its bridge {bridge} um must be narrower than its segments"),
            ));
        }
        let n = self.periods;
        let seg = self.duty * a;
        let end = (n - 1) as f64 * a + seg;
        let mut d = Drawing::new(k);
        if bridge > 0.0 {
            d.add(layer, strip(bridge, 0.0, 0.0, end)?);
        }
        let segments: Vec<Element> = (0..n)
            .map(|i| {
                let f = if n == 1 {
                    0.0
                } else {
                    i as f64 / (n - 1) as f64
                };
                let w = w0 + (w1 - w0) * f;
                let x0 = i as f64 * a;
                Ok(Element::new([i as i64, 0], strip(w, 0.0, x0, x0 + seg)?))
            })
            .collect::<Result<_>>()?;
        d.add(
            layer,
            Shape::Array(Array::new(segments).map_err(|e| refuse(k, reason(e)))?),
        );
        d.add_port(Port::new("o1", Point::um(0.0, 0.0), 180.0, um(w0), layer))?;
        d.add_port(Port::new("o2", Point::um(end, 0.0), 0.0, um(w1), layer))?;
        Ok(d)
    }
}

/// A grating coupler: an access waveguide's end at the origin, a linear taper along x from its
/// `width` to the grating's `grating_width` over `taper_length`, then `periods` teeth, each the
/// first `fill` of its `period` (unetched), across the grating's width, the trenches between
/// them etched `etch` deep.
///
/// Apodized: the fill F(x), an expression of the tooth's start x (from the grating's start).
/// Drawn on one layer, the teeth are etched partly, and the stack leaves the layer's material
/// below their floor across the whole layer (docs/methods/stacks.md). With a `slab_layer` the
/// grating is two layers instead: the teeth fully etched on `layer` (the top, as thick as the
/// etch), and under them on `slab_layer` the slab, the taper and the guide; the taper is drawn
/// on both. Port `o1` (0, 0) facing 180°, `width` wide.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GratingCoupler {
    /// The access waveguide's width.
    pub width: Length,
    /// The grating's width, across y.
    pub grating_width: Length,
    /// The taper's length.
    pub taper_length: Length,
    /// The period, Λ.
    pub period: Length,
    /// The fraction of each period a tooth spans, from 0 to 1.
    pub fill: f64,
    /// How many teeth.
    pub periods: usize,
    /// The trenches' depth (a partial etch on one layer).
    pub etch: Length,
    /// The fill F(x), an expression of each tooth's start `x` (from the grating's start) and
    /// of the parameters given with it; `None`: uniform.
    pub apodization: Option<(Expr, Parameters)>,
    /// The layer of the slab under the teeth; `None`: one layer, partly etched.
    pub slab_layer: Option<String>,
}

impl GratingCoupler {
    /// A uniform grating coupler.
    pub fn new(
        width: Length,
        grating_width: Length,
        taper_length: Length,
        period: Length,
        fill: f64,
        periods: usize,
        etch: Length,
    ) -> GratingCoupler {
        GratingCoupler {
            width,
            grating_width,
            taper_length,
            period,
            fill,
            periods,
            etch,
            apodization: None,
            slab_layer: None,
        }
    }

    /// The same coupler, its fill `apodization`, F(x).
    #[must_use]
    pub fn with_apodization(mut self, apodization: Expr, parameters: Parameters) -> GratingCoupler {
        self.apodization = Some((apodization, parameters));
        self
    }

    /// The same coupler on two layers: its slab on `slab_layer`.
    #[must_use]
    pub fn with_slab_layer(mut self, slab_layer: &str) -> GratingCoupler {
        self.slab_layer = Some(slab_layer.to_owned());
        self
    }
}

impl Generator for GratingCoupler {
    fn kind(&self) -> &str {
        "grating coupler"
    }

    fn draw(&self, layer: &str) -> Result<Drawing> {
        let k = self.kind();
        let w = positive(k, "width", self.width, false)?;
        let gw = positive(k, "grating_width", self.grating_width, false)?;
        let lt = positive(k, "taper_length", self.taper_length, false)?;
        let a = positive(k, "period", self.period, false)?;
        let depth = positive(k, "etch", self.etch, false)?;
        if self.periods == 0 {
            return Err(refuse(k, "it needs at least one tooth"));
        }
        let fill = |x: f64| -> Result<f64> {
            let f = match &self.apodization {
                None => self.fill,
                Some((e, p)) => at_x(e, p, x, k, "fill")?,
            };
            if f > 0.0 && f < 1.0 {
                Ok(f)
            } else {
                Err(refuse(
                    k,
                    format!("its fill at x = {x} um is {f}: it must be between 0 and 1"),
                ))
            }
        };
        let taper = Shape::Polygon(Polygon::new(vec![
            Point::um(0.0, -w / 2.0),
            Point::um(lt, -gw / 2.0),
            Point::um(lt, gw / 2.0),
            Point::um(0.0, w / 2.0),
        ])?);
        let l = a * self.periods as f64;
        let mut teeth = Vec::with_capacity(self.periods);
        for i in 0..self.periods {
            let x = i as f64 * a;
            let t = fill(x)? * a;
            teeth.push(Element::new(
                [i as i64, 0],
                strip(gw, 0.0, lt + x, lt + x + t)?,
            ));
        }
        let teeth = Shape::Array(Array::new(teeth).map_err(|e| refuse(k, reason(e)))?);
        let mut d = Drawing::new(k);
        match &self.slab_layer {
            None => {
                d.add(layer, taper);
                d.add_etched(layer, teeth, Etch::partial(um(depth)));
            }
            Some(slab) => {
                d.add(slab, taper.clone());
                d.add(slab, strip(gw, 0.0, lt, lt + l)?);
                d.add(layer, taper);
                d.add(layer, teeth);
            }
        }
        d.add_port(Port::new("o1", Point::um(0.0, 0.0), 180.0, um(w), layer))?;
        Ok(d)
    }
}

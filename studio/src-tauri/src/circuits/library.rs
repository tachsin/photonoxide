//! The component library the studio offers: every kind of component a chip can place, under a
//! stable id that chip files name, with what the Components page shows beside the library's own
//! description of it (a title, a category, a sentence, the model's equation) and the schematic
//! symbol the chip view draws.
//!
//! The physics is the library's (`photonoxide::circuit::components`): each entry builds one of
//! its components, and everything about its ports, parameters and provenance is read from that
//! component. What the studio chooses is what the library leaves to its caller: the guide every
//! waveguide, bend and ring carries, and the MMIs' widths.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use photonoxide::Complex64 as c64;
use photonoxide::Result;
use photonoxide::circuit::components::{
    self, AddDropRing, AllPassRing, Bend, Coupler, DirectionalCoupler, Dispersion, Mmi, MmiKind,
    PhaseShifter, Planar, Supermodes, Waveguide, YBranch,
};
use photonoxide::circuit::{Component, Fixed, Port, SMatrix, Spectrum};
use photonoxide::compact::Measured;
use photonoxide::compact::touchstone::{Convention, Touchstone};
use photonoxide::material;
use photonoxide::mode::Polarization;
use photonoxide::mode::vector::{Boundaries, Boundary, CrossSection, Permittivity};
use photonoxide::units::{Length, Wavelength};
use serde::Serialize;

/// A kind of component in the library.
#[derive(Clone, Copy)]
pub struct Entry {
    /// The id chip files name it by, e.g. `"waveguide"`. Stable: renaming one breaks files.
    pub id: &'static str,
    pub title: &'static str,
    /// Which model of its kind it is, when the kind has several (e.g. `"ideal"`), or `""`.
    pub variant: &'static str,
    /// The library's section, e.g. `"Waveguides"`.
    pub category: &'static str,
    /// One sentence on what it is.
    pub about: &'static str,
    /// Its model's equations in TeX (display math), or `""`.
    pub equation: &'static str,
    pub glyph: Glyph,
    /// The component: built once and shared, as a netlist shares its models.
    pub build: fn() -> Built,
}

/// A component, or why it couldn't be built.
pub type Built = std::result::Result<Arc<dyn Component>, String>;

/// The schematic symbol's drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Glyph {
    Waveguide,
    Bend,
    PhaseShifter,
    Coupler,
    Mmi,
    YBranch,
    RingAllPass,
    RingAddDrop,
    Mzi,
    Terminator,
    Measured,
    Box,
}

/// A component's schematic symbol: its size and where each port sits on it, in the chip view's
/// units (its grid is 10), centred on the origin, y pointing down.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Symbol {
    pub glyph: Glyph,
    pub width: f64,
    pub height: f64,
    /// One per port, in the component's order.
    pub pins: Vec<Pin>,
}

/// Where a port sits on its symbol, and the direction a wire leaves it in (degrees, 0 along +x,
/// 90 along +y, which is down).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Pin {
    pub port: String,
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

/// Builds a component the first time it is asked for, and shares it after.
macro_rules! once {
    ($body:expr) => {{
        static CELL: OnceLock<Built> = OnceLock::new();
        CELL.get_or_init(|| -> Built { $body }).clone()
    }};
}

fn text(e: photonoxide::Error) -> String {
    e.to_string()
}

/// The guide every waveguide, bend and ring of the library carries: a 500 × 220 nm silicon
/// strip in oxide, its TE-like mode's n_eff 2.44506 and n_g 4.17290 at 1.55 µm by photonoxide's
/// full-vector solver (examples/group_index.rs, on a 6.25 × 5 nm grid; Chrostowski & Hochberg
/// 2015 give 4.18), losing 2.7 dB/cm to start (Bogaerts et al. 2011's ring waveguide, Section 2.4; a straight
/// waveguide's loss is a parameter).
fn strip() -> std::result::Result<Dispersion, String> {
    Ok(Dispersion::new(Wavelength::um(1.55).map_err(text)?, 2.44506, 4.172901).with_loss(2.7))
}

/// The plane the MMIs lie in: a 220 nm silicon film in oxide, by the effective index method.
fn soi() -> Planar {
    Planar::Film {
        core: material::silicon(),
        cladding: material::silica(),
        thickness: Length::nm(220.0),
        vertical: Polarization::Te,
    }
}

fn waveguide() -> Built {
    once!(Ok(Arc::new(Waveguide::new(strip()?))))
}

/// The strip's cross-section at `wavelength`: a quarter of it (x, y ≥ 0, the strip's centre at
/// the origin), 500 × 220 nm of Li's silicon in oxide of index 1.444, on a 12.5 × 10 nm grid,
/// with an electric wall at x = 0 and a magnetic wall at y = 0 for the TE-like mode.
fn strip_section(wavelength: Wavelength) -> Result<CrossSection> {
    let si = material::silicon().permittivity(wavelength)?;
    let ox = c64::new(1.444 * 1.444, 0.0);
    CrossSection::uniform((0.0, 1.05, 84), (0.0, 0.75, 75), |x, y| {
        Permittivity::isotropic(if x < 0.25 && y < 0.11 { si } else { ox })
    })?
    .with_boundaries(Boundaries {
        west: Boundary::ElectricWall,
        south: Boundary::MagneticWall,
        ..Boundaries::default()
    })
}

fn waveguide_modes() -> Built {
    once!(Ok(Arc::new(
        Waveguide::from_modes(
            strip_section,
            Wavelength::um(1.55).map_err(text)?,
            0.025,
            None
        )
        .map_err(text)?
    )))
}

fn bend() -> Built {
    once!(Ok(Arc::new(Bend::new(10.0, strip()?).map_err(text)?)))
}

fn phase_shifter() -> Built {
    once!(Ok(Arc::new(PhaseShifter::new())))
}

fn coupler() -> Built {
    once!(Ok(Arc::new(Coupler::new())))
}

fn directional_coupler() -> Built {
    // two strips 200 nm apart: their supermodes 0.02086 apart at 1.55 µm by photonoxide's
    // full-vector solver (examples/directional_coupler.rs, a 5 nm grid), so C = πΔn/λ =
    // 0.04228 /µm and a cross-over length of 37.2 µm (Chrostowski & Hochberg 2015: 37.5 µm)
    once!(Ok(Arc::new(
        DirectionalCoupler::new(Supermodes::from_coupling(strip()?, 0.042_28)).map_err(text)?
    )))
}

fn mmi(kind: MmiKind) -> Built {
    let wavelength = Wavelength::um(1.55).map_err(text)?;
    Ok(Arc::new(
        Mmi::new(kind, soi(), 6.0, 1.5, wavelength).map_err(text)?,
    ))
}

fn mmi_1x2() -> Built {
    once!(mmi(MmiKind::OneByTwo))
}

fn mmi_2x2() -> Built {
    once!(mmi(MmiKind::TwoByTwo))
}

fn y_branch() -> Built {
    once!(Ok(Arc::new(YBranch::new())))
}

fn ring_all_pass() -> Built {
    once!(Ok(Arc::new(AllPassRing::new(strip()?).map_err(text)?)))
}

fn ring_add_drop() -> Built {
    once!(Ok(Arc::new(AddDropRing::new(strip()?).map_err(text)?)))
}

fn mzi() -> Built {
    // the library's own MZI netlist, of ideal 3 dB couplers and strip arms 50 µm apart
    once!({
        let coupler: Arc<dyn Component> = Arc::new(Coupler::new());
        let arm: Arc<dyn Component> = Arc::new(Waveguide::new(strip()?));
        let build = || -> Result<Arc<dyn Component>> {
            let mut netlist = components::mzi(coupler.clone(), coupler, arm.clone(), arm)?;
            netlist.set("upper", "length", 150.0)?;
            netlist.set("lower", "length", 100.0)?;
            Ok(Arc::new(netlist.compile()?))
        };
        build().map_err(text)
    })
}

fn terminator() -> Built {
    once!(Ok(Arc::new(
        Fixed::new("terminator", &["o1"], SMatrix::zeros(1)).map_err(text)?
    )))
}

/// Every kind of component in the library, in the order the library lists them.
pub fn entries() -> Vec<Entry> {
    vec![
        Entry {
            id: "waveguide",
            title: "Waveguide",
            variant: "stated indices",
            category: "Waveguides",
            about: "A straight strip, 500 × 220 nm of silicon in oxide: its TE-like mode's phase (n_eff 2.445 and n_g 4.173 at 1.55 µm, by photonoxide's mode solver), and its loss, 2.7 dB/cm to start.",
            equation: r"S_{21} = S_{12} = e^{i\gamma L},\quad \gamma = \frac{2\pi n(\lambda)}{\lambda} + \frac{i\alpha}{2},\quad n(\lambda) = n_0 + \frac{n_0 - n_g}{\lambda_0}\,(\lambda - \lambda_0)",
            glyph: Glyph::Waveguide,
            build: waveguide,
        },
        Entry {
            id: "waveguide-modes",
            title: "Waveguide",
            variant: "mode solver",
            category: "Waveguides",
            about: "The same strip, its mode found by photonoxide's full-vector solver at five wavelengths around 1.55 µm (a 12.5 × 10 nm grid): a 3D model, valid from 1.5 to 1.6 µm, without loss.",
            equation: r"S_{21} = S_{12} = e^{i\gamma L},\quad n(\lambda) = n_0 + n'(\lambda - \lambda_0) + \tfrac{1}{2} n''(\lambda - \lambda_0)^2",
            glyph: Glyph::Waveguide,
            build: waveguide_modes,
        },
        Entry {
            id: "bend",
            title: "Bend",
            variant: "",
            category: "Waveguides",
            about: "A circular bend of 10 µm radius in the same strip, its angle a parameter: the straight mode's indices along the arc, without the transitions to straight guides.",
            equation: r"S_{21} = S_{12} = e^{i\gamma R\theta}",
            glyph: Glyph::Bend,
            build: bend,
        },
        Entry {
            id: "phase-shifter",
            title: "Phase shifter",
            variant: "",
            category: "Waveguides",
            about: "An ideal phase shifter: a phase φ added to the light passing, nothing lost, the same at every wavelength. A heater or a modulator, as a circuit sees it.",
            equation: r"S_{21} = S_{12} = e^{i\varphi}",
            glyph: Glyph::PhaseShifter,
            build: phase_shifter,
        },
        Entry {
            id: "coupler",
            title: "Coupler",
            variant: "ideal",
            category: "Couplers and splitters",
            about: "An ideal 2 × 2 coupler: a share κ² of the power crosses to the other guide, the rest goes through, losslessly.",
            equation: r"\begin{pmatrix} b_4 \\ b_3 \end{pmatrix} = \begin{pmatrix} t & i\kappa \\ i\kappa & t \end{pmatrix} \begin{pmatrix} a_1 \\ a_2 \end{pmatrix},\quad t = \sqrt{1 - \kappa^2}",
            glyph: Glyph::Coupler,
            build: coupler,
        },
        Entry {
            id: "directional-coupler",
            title: "Directional coupler",
            variant: "coupled modes",
            category: "Couplers and splitters",
            about: "Two strips 200 nm apart, by coupled-mode theory from their two supermodes (37.2 µm cross-over length at 1.55 µm, by photonoxide's mode solver): the power crosses over and back along the coupler's length.",
            equation: r"t = \cos\frac{\pi\,\Delta n\,L}{\lambda},\quad \kappa = \sin\frac{\pi\,\Delta n\,L}{\lambda},\quad \Delta n = n_\mathrm{even} - n_\mathrm{odd}",
            glyph: Glyph::Coupler,
            build: directional_coupler,
        },
        Entry {
            id: "mmi-1x2",
            title: "MMI 1 × 2",
            variant: "",
            category: "Couplers and splitters",
            about: "A 1 × 2 multimode interference splitter on 220 nm SOI, 6 µm wide with 1.5 µm access guides, by guided-mode propagation analysis.",
            equation: r"L_\pi = \frac{\pi}{\beta_0 - \beta_1},\qquad L = \frac{3 L_\pi}{8}",
            glyph: Glyph::Mmi,
            build: mmi_1x2,
        },
        Entry {
            id: "mmi-2x2",
            title: "MMI 2 × 2",
            variant: "",
            category: "Couplers and splitters",
            about: "A 2 × 2 multimode interference 3 dB coupler on 220 nm SOI, 6 µm wide with 1.5 µm access guides, by guided-mode propagation analysis.",
            equation: r"L_\pi = \frac{\pi}{\beta_0 - \beta_1},\qquad L = \frac{L_\pi}{2}",
            glyph: Glyph::Mmi,
            build: mmi_2x2,
        },
        Entry {
            id: "y-branch",
            title: "Y-branch",
            variant: "",
            category: "Couplers and splitters",
            about: "An ideal 50/50 splitter with an excess loss: the stem's power split equally between the arms.",
            equation: r"S_{21} = S_{31} = \frac{10^{-A/20}}{\sqrt{2}}",
            glyph: Glyph::YBranch,
            build: y_branch,
        },
        Entry {
            id: "ring-all-pass",
            title: "All-pass ring",
            variant: "",
            category: "Resonators",
            about: "A ring of the strip coupled to one bus, by Bogaerts et al.'s closed form: the bus's transmission dips at each resonance.",
            equation: r"T = \frac{a^2 - 2ra\cos\phi + r^2}{1 - 2ar\cos\phi + (ra)^2}",
            glyph: Glyph::RingAllPass,
            build: ring_all_pass,
        },
        Entry {
            id: "ring-add-drop",
            title: "Add-drop ring",
            variant: "",
            category: "Resonators",
            about: "A ring of the strip between two buses, by Bogaerts et al.'s closed forms: on resonance the light leaves by the drop port.",
            equation: r"T_p = \frac{r_2^2 a^2 - 2 r_1 r_2 a \cos\phi + r_1^2}{1 - 2 r_1 r_2 a \cos\phi + (r_1 r_2 a)^2},\quad T_d = \frac{(1 - r_1^2)(1 - r_2^2)\, a}{1 - 2 r_1 r_2 a \cos\phi + (r_1 r_2 a)^2}",
            glyph: Glyph::RingAddDrop,
            build: ring_add_drop,
        },
        Entry {
            id: "mzi",
            title: "Mach–Zehnder interferometer",
            variant: "",
            category: "Interferometers",
            about: "The library's own MZI: a circuit of two ideal 3 dB couplers and two strip arms (150 and 100 µm to start), solved as a netlist.",
            equation: r"S_{o3,o2} = t_s t_c\, e^{i\gamma L_u} + (i\kappa_s)(i\kappa_c)\, e^{i\gamma L_l}",
            glyph: Glyph::Mzi,
            build: mzi,
        },
        Entry {
            id: "terminator",
            title: "Terminator",
            variant: "",
            category: "Terminations",
            about: "A perfect absorber: closes a port that light reaching it should leave by.",
            equation: r"S = 0",
            glyph: Glyph::Terminator,
            build: terminator,
        },
    ]
}

/// The entry with id `id`.
pub fn entry(id: &str) -> Option<Entry> {
    entries().into_iter().find(|e| e.id == id)
}

/// The entry `component` is, for a netlist built outside the studio: the entry that built it
/// (the library's components are built once and shared), else the first of its kind
/// (`Component::kind`) with the same provenance, else the first of its kind.
#[cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "for `Chip::from_netlist`, which only the tests take yet"
    )
)]
pub fn entry_of(component: &Arc<dyn Component>) -> Option<Entry> {
    let built: Vec<(Entry, Arc<dyn Component>)> = entries()
        .into_iter()
        .filter_map(|e| (e.build)().ok().map(|c| (e, c)))
        .collect();
    let kind = component.kind();
    let same = |c: &Arc<dyn Component>| c.kind() == kind;
    built
        .iter()
        .find(|(_, c)| Arc::ptr_eq(c, component))
        .or_else(|| {
            built
                .iter()
                .find(|(_, c)| same(c) && c.provenance() == component.provenance())
        })
        .or_else(|| built.iter().find(|(_, c)| same(c)))
        .map(|(e, _)| *e)
}

/// A glyph's drawing for a number of ports: its width, its height, and each port's place and
/// direction, `(x, y, angle)`, in port order.
type Layout = (f64, f64, &'static [(f64, f64, f64)]);

/// The symbol for `glyph` with `ports`: the glyph's own layout when it has one for that many
/// ports, else a box with the first half of the ports on its left and the rest on its right.
/// The layouts follow the library's port conventions: a 2 × 2 device's `o1` lower left, `o2`
/// upper left, `o3` upper right, `o4` lower right; a 1 × 2 device's `o1` its input, `o2` its
/// upper output; a ring's bus `in` to `through`, its drop bus `add` to `drop`.
pub fn symbol(glyph: Glyph, ports: &[Port]) -> Symbol {
    let layout: Option<Layout> = match (glyph, ports.len()) {
        (Glyph::Waveguide | Glyph::PhaseShifter, 2) => {
            Some((60.0, 20.0, &[(-30.0, 0.0, 180.0), (30.0, 0.0, 0.0)]))
        }
        (Glyph::Bend, 2) => Some((40.0, 40.0, &[(-20.0, 10.0, 180.0), (10.0, -20.0, 270.0)])),
        (Glyph::Coupler | Glyph::Mmi | Glyph::Mzi, 4) => Some((
            80.0,
            40.0,
            &[
                (-40.0, 10.0, 180.0),
                (-40.0, -10.0, 180.0),
                (40.0, -10.0, 0.0),
                (40.0, 10.0, 0.0),
            ],
        )),
        (Glyph::Mmi | Glyph::YBranch, 3) => Some((
            60.0,
            40.0,
            &[(-30.0, 0.0, 180.0), (30.0, -10.0, 0.0), (30.0, 10.0, 0.0)],
        )),
        (Glyph::Mzi, 2) => Some((80.0, 40.0, &[(-40.0, 0.0, 180.0), (40.0, 0.0, 0.0)])),
        (Glyph::RingAllPass, 2) => Some((60.0, 60.0, &[(-30.0, 20.0, 180.0), (30.0, 20.0, 0.0)])),
        (Glyph::RingAddDrop, 4) => Some((
            60.0,
            80.0,
            &[
                (-30.0, 30.0, 180.0),
                (30.0, 30.0, 0.0),
                (30.0, -30.0, 0.0),
                (-30.0, -30.0, 180.0),
            ],
        )),
        (Glyph::Terminator, 1) => Some((20.0, 20.0, &[(-10.0, 0.0, 180.0)])),
        _ => None,
    };
    if let Some((width, height, at)) = layout {
        return Symbol {
            glyph,
            width,
            height,
            pins: ports
                .iter()
                .zip(at)
                .map(|(p, &(x, y, angle))| Pin {
                    port: p.name.clone(),
                    x,
                    y,
                    angle,
                })
                .collect(),
        };
    }
    let left = ports.len().div_ceil(2);
    let right = ports.len() - left;
    let rows = left.max(right).max(1);
    let height = 20.0 * rows as f64;
    let width = 60.0;
    let pins = ports
        .iter()
        .enumerate()
        .map(|(k, p)| {
            let (side, row, count) = if k < left {
                (-1.0, k, left)
            } else {
                (1.0, k - left, right)
            };
            Pin {
                port: p.name.clone(),
                x: side * width / 2.0,
                y: 20.0 * row as f64 - 10.0 * (count as f64 - 1.0),
                angle: if side < 0.0 { 180.0 } else { 0.0 },
            }
        })
        .collect();
    Symbol {
        glyph: if glyph == Glyph::Measured {
            Glyph::Measured
        } else {
            Glyph::Box
        },
        width,
        height,
        pins,
    }
}

/// The measured component in the Touchstone file at `path`, its values in `convention`: the
/// library's measured component (`photonoxide::compact::Measured`), ports `o1`, `o2`, … in
/// the file's order, interpolated between the file's wavelengths and refused outside them.
///
/// # Errors
///
/// A message for a file that can't be read, isn't S-parameters, or doesn't make a spectrum.
pub fn measured(path: &Path, convention: Convention) -> Built {
    let read = || -> Result<Arc<dyn Component>> {
        let spectrum = Touchstone::read(path)?.spectrum(convention)?;
        // λ = c/f loses the last bit of a wavelength written as c/λ, so each is rounded to
        // the femtometre: a sweep from 1.53 µm then starts inside the file's band, not 2e-16 µm
        // before it
        let wavelengths = spectrum
            .wavelengths()
            .iter()
            .map(|w| Wavelength::um((w.to_um() * 1e9).round() / 1e9))
            .collect::<Result<Vec<_>>>()?;
        let spectrum = Spectrum::new(
            spectrum.ports().to_vec(),
            wavelengths,
            spectrum.matrices().to_vec(),
        )?;
        let c: Arc<dyn Component> = Arc::new(Measured::new(&spectrum, path.display().to_string())?);
        Ok(c)
    };
    read().map_err(|e| format!("{}: {e}", path.display()))
}

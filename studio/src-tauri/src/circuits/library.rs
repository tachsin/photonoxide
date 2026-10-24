//! The component library the studio offers: every kind of component a chip can place, under a
//! stable id that chip files name, with what the Components page shows beside the library's own
//! description of it (a title, a category, a sentence, the model's equation) and the schematic
//! symbol the chip view draws.
//!
//! The physics is the library's: the waveguide, the bend, the directional coupler, the rings,
//! the rib, the MMIs, the Y-branch and the MZIs of couplers and of Y-branches are its devices
//! (`photonoxide::device`) at their defaults, each the component its cheapest model gives (the
//! analytic one; the MMIs' and the rib's 2D; the strip waveguide's 3D model for
//! `waveguide-modes`); the others are its components (`photonoxide::circuit::components`).
//! Everything about their ports, parameters and provenance is read from the component. What the
//! studio chooses is what the library leaves to its caller: the window `waveguide-modes` is
//! solved on.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use photonoxide::Complex64 as c64;
use photonoxide::Result;
use photonoxide::circuit::components::{self, Coupler, PhaseShifter};
use photonoxide::circuit::{Component, Fidelity, Fixed, Port, SMatrix, Spectrum};
use photonoxide::compact::Measured;
use photonoxide::compact::touchstone::{Convention, Touchstone};
use photonoxide::device::{
    Device, Mesh, RibWaveguide, Scheme, StripBend, StripCoupler, StripMmi, StripMzi, StripRing,
    StripWaveguide, StripYBranch, ThermoShifter, Values,
};
use photonoxide::material::{self, Material, Model, Provenance};
use photonoxide::stack::{Layer, LayerStack};
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

/// A device's component at `fidelity` and `values`, to be shared.
fn component(device: &dyn Device, values: &Values, fidelity: Fidelity) -> Built {
    let built = device.component(values, fidelity).map_err(text)?;
    Ok(Arc::from(built.component))
}

/// A device's analytic model at its defaults: for every waveguide, bend, coupler and ring, a
/// 500 × 220 nm silicon strip in oxide, its TE-like mode's n_eff 2.44506 and n_g 4.17290 at
/// 1.55 µm by photonoxide's full-vector solver (examples/group_index.rs, on a 6.25 × 5 nm grid;
/// Chrostowski & Hochberg 2015 give 4.18), losing 2.7 dB/cm to start (Bogaerts et al. 2011's
/// ring waveguide, Section 2.4; a straight waveguide's loss is a parameter).
fn analytic(device: &dyn Device) -> Built {
    component(device, &Values::defaults(device), Fidelity::Analytic)
}

fn waveguide() -> Built {
    once!(analytic(&StripWaveguide::new()))
}

/// Li's silicon in Chrostowski & Hochberg's oxide, of constant index 1.444: the stack
/// `waveguide-modes` is solved on.
fn oxide_stack() -> Result<LayerStack> {
    let oxide = Material::new(
        "oxide (1.444)",
        Model::Constant(c64::new(1.444, 0.0)),
        Wavelength::um(1.2)?,
        Wavelength::um(2.0)?,
        Provenance {
            reference: "L. Chrostowski, M. Hochberg, Silicon Photonics Design (2015), Fig. 3.14"
                .into(),
            doi: "10.1017/CBO9781316084168".into(),
            data: "the book's oxide index".into(),
            temperature: None,
            notes: String::new(),
        },
    )?;
    let layer = |name: &str, thickness: Length, material: Material| Layer {
        name: name.into(),
        thickness,
        material,
        background: oxide.clone(),
    };
    LayerStack::new(
        oxide.clone(),
        vec![
            layer("BOX", Length::um(2.0), oxide.clone()),
            layer("Si", Length::nm(220.0), material::silicon()),
        ],
        oxide.clone(),
    )
}

/// The strip's 3D model: a quarter of its cross-section (x, y ≥ 0, the strip's centre at the
/// origin), 500 × 220 nm of Li's silicon in oxide of index 1.444, 1.05 × 0.75 µm on a 12.5 ×
/// 10 nm grid, with an electric wall at x = 0 and a magnetic wall at y = 0 for the TE-like
/// mode; its mode at five wavelengths 25 nm apart about 1.55 µm, by the standard scheme.
fn waveguide_modes() -> Built {
    once!({
        let device = StripWaveguide::on(oxide_stack().map_err(text)?, "Si")
            .map_err(text)?
            .with_mesh(Mesh::window(2.1, 1.5, 84, 75, Scheme::Standard));
        component(&device, &Values::defaults(&device), Fidelity::ThreeD)
    })
}

fn bend() -> Built {
    once!(analytic(&StripBend::new()))
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
    once!(analytic(&StripCoupler::new()))
}

/// A device's 2D model at its defaults: the MMIs' guided modes on a 220 nm silicon film in
/// oxide by the effective index method, 6 µm wide with 1.5 µm access guides; the rib's
/// effective index method.
fn two_d(device: &dyn Device) -> Built {
    component(device, &Values::defaults(device), Fidelity::TwoD)
}

fn rib_waveguide() -> Built {
    once!(two_d(&RibWaveguide::new()))
}

fn mmi_1x2() -> Built {
    once!(two_d(&StripMmi::one_by_two()))
}

fn mmi_2x2() -> Built {
    once!(two_d(&StripMmi::two_by_two()))
}

fn y_branch() -> Built {
    once!(analytic(&StripYBranch::new()))
}

fn mzi_couplers() -> Built {
    once!(analytic(&StripMzi::couplers()))
}

fn mzi_y_branches() -> Built {
    once!(analytic(&StripMzi::y_branches()))
}

fn thermo_metal() -> Built {
    once!(analytic(&ThermoShifter::metal()))
}

fn thermo_doped() -> Built {
    once!(analytic(&ThermoShifter::doped()))
}

fn ring_all_pass() -> Built {
    once!(analytic(&StripRing::all_pass()))
}

fn ring_add_drop() -> Built {
    once!(analytic(&StripRing::add_drop()))
}

fn mzi() -> Built {
    // the library's own MZI netlist, of ideal 3 dB couplers and strip arms 50 µm apart
    once!({
        let coupler: Arc<dyn Component> = Arc::new(Coupler::new());
        let arm = analytic(&StripWaveguide::new())?;
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
            id: "rib-waveguide",
            title: "Rib waveguide",
            variant: "",
            category: "Waveguides",
            about: "A rib 500 nm wide on 220 nm SOI, etched 130 nm into the silicon and standing on a 90 nm slab: its TE-like mode by the effective index method (a 2D approximation), losing 2.7 dB/cm to start.",
            equation: r"S_{21} = S_{12} = e^{i\gamma L},\quad n_\mathrm{eff}:\ n_\mathrm{rib},\ n_\mathrm{slab} \to \text{the lateral slab}",
            glyph: Glyph::Waveguide,
            build: rib_waveguide,
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
            about: "Zhang et al.'s Y-junction (a 2 µm stem widening along a spline to two 500 nm arms), modelled as an ideal 50/50 splitter with an excess loss: the stem's power split equally between the arms.",
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
            id: "mzi-couplers",
            title: "Mach–Zehnder interferometer",
            variant: "directional couplers",
            category: "Interferometers",
            about: "An MZI of two directional couplers (500 nm strips 200 nm apart, each at its 3 dB length) and strip arms 150 and 100 µm long, a phase in the upper arm: the netlist of its devices.",
            equation: r"S_{o3,o1} = x_c\, e^{i(\gamma L_u + \varphi)}\, t_s + t_c\, e^{i\gamma L_l}\, x_s",
            glyph: Glyph::Mzi,
            build: mzi_couplers,
        },
        Entry {
            id: "mzi-y-branches",
            title: "Mach–Zehnder interferometer",
            variant: "Y-branches",
            category: "Interferometers",
            about: "An MZI of two Y-branches and strip arms 150 and 100 µm long, a phase in the upper arm: the netlist of its devices; the light out of phase at the combiner radiates.",
            equation: r"S_{21} = \frac{10^{-A/10}}{2} \left(e^{i(\gamma L_u + \varphi)} + e^{i\gamma L_l}\right)",
            glyph: Glyph::Mzi,
            build: mzi_y_branches,
        },
        Entry {
            id: "thermo-optic-shifter",
            title: "Thermo-optic phase shifter",
            variant: "metal heater",
            category: "Active",
            about: "Jacques et al.'s TiN heater 7.5 µm wide, 2 µm over a 500 × 220 nm strip, 320 µm long: the phase its power adds, P_π by its closed forms (the oxide as a slab, Marcatili's dn_eff/dT).",
            equation: r"\varphi = \pi \frac{P}{P_\pi},\qquad P_\pi = \frac{\lambda}{2\,\partial n_\mathrm{eff}/\partial P'}",
            glyph: Glyph::PhaseShifter,
            build: thermo_metal,
        },
        Entry {
            id: "doped-thermo-optic-shifter",
            title: "Thermo-optic phase shifter",
            variant: "doped heaters",
            category: "Active",
            about: "Jacques et al.'s N++ silicon heaters, 1 µm strips in the 90 nm slab 0.8 µm beyond a 500 × 220 nm rib, 320 µm long: the phase their power adds, P_π by its closed forms.",
            equation: r"\varphi = \pi \frac{P}{P_\pi},\qquad P_\pi = \frac{\lambda}{2\,\partial n_\mathrm{eff}/\partial P'}",
            glyph: Glyph::PhaseShifter,
            build: thermo_doped,
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

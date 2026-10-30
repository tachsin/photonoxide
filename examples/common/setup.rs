//! What an example prints first: its setup, every dimension and material of what it computes.
//!
//! An example builds a [`Setup`] from the same values its computation uses (its stack, its
//! shapes, its materials, its grid) and prints it before it computes anything, so the block
//! can't drift from the computation. The layers are listed from the top down, the bottom one
//! (the substrate) the first laid down in fabrication, whatever order a stack keeps them in
//! (photonoxide's [`LayerStack`] keeps them bottom to top).
//!
//! `PHOTONOXIDE_SETUP_ONLY=1` makes an example print its setup and stop (exit code 0), before
//! the computation: a long run's setup without the run.

use photonoxide::circuit::{Circuit, Component};
use photonoxide::device::{Device, Values};
use photonoxide::geometry::Shape;
use photonoxide::material::{Material, Model, Orientation};
use photonoxide::stack::{LayerStack, Structure};
use photonoxide::units::Wavelength;

/// The environment variable that makes an example print its setup and stop.
pub const SETUP_ONLY: &str = "PHOTONOXIDE_SETUP_ONLY";

/// An example's setup, printed by [`Setup::print`]: for each of its parts (one, or a part per
/// structure it computes), the layers from the top down, the shapes on them and its settings
/// (the wavelength, the window, the grid, the solver, the tolerance, in the order given); then
/// its materials, each with its model, its source and its range.
#[derive(Default)]
pub struct Setup {
    parts: Vec<Part>,
    materials: Vec<String>,
    named: Vec<String>,
}

/// One structure of an example's, or the whole example's setup.
#[derive(Default)]
struct Part {
    title: String,
    layers: Vec<String>,
    shapes: Vec<String>,
    items: Vec<(String, String)>,
}

impl Setup {
    /// An empty setup.
    pub fn new() -> Setup {
        Setup::default()
    }

    /// The part being written.
    fn last(&mut self) -> &mut Part {
        if self.parts.is_empty() {
            self.parts.push(Part::default());
        }
        self.parts.last_mut().expect("a part")
    }

    /// Starts a part of its own, `title`: the layers, shapes and settings after it are its.
    /// What came before (if anything) is the whole example's.
    pub fn part(&mut self, title: &str) {
        self.parts.push(Part {
            title: title.to_owned(),
            ..Part::default()
        });
    }

    /// A layer, called from the top down: its name, its thickness in µm (`None` for a
    /// half-space: a cladding above, a substrate below), what fills it, and its index or
    /// permittivity ([`n`], [`eps`]) at the example's wavelength.
    pub fn layer(&mut self, name: &str, thickness_um: Option<f64>, material: &str, optical: &str) {
        let thickness = thickness_um.map_or_else(|| "half-space".to_owned(), length);
        self.layer_sized(name, &thickness, material, optical);
    }

    /// A layer whose thickness is a range or a list, called from the top down, as
    /// [`Setup::layer`].
    pub fn layer_sized(&mut self, name: &str, thickness: &str, material: &str, optical: &str) {
        self.last()
            .layers
            .push(format!("{name:<14} {thickness:<12} {material}, {optical}"));
    }

    /// The layers of `stack`, from the top down (its cladding first, its substrate last), each
    /// material's index at `wavelength`, and every material among the materials.
    pub fn stack(&mut self, stack: &LayerStack, wavelength: Wavelength) {
        let index = |m: &Material| index_at(m, wavelength);
        let cladding = stack.cladding();
        self.layer("cladding", None, cladding.name(), &index(cladding));
        for layer in stack.layers().iter().rev() {
            let filled = if layer.material == layer.background {
                format!("{}, {}", layer.material.name(), index(&layer.material))
            } else {
                format!(
                    "{}, {} in its shapes; {}, {} elsewhere",
                    layer.material.name(),
                    index(&layer.material),
                    layer.background.name(),
                    index(&layer.background)
                )
            };
            let thickness = length(layer.thickness.to_um());
            self.last()
                .layers
                .push(format!("{:<14} {thickness:<12} {filled}", layer.name));
        }
        let substrate = stack.substrate();
        self.layer("substrate", None, substrate.name(), &index(substrate));
        self.material(cladding, wavelength);
        for layer in stack.layers().iter().rev() {
            self.material(&layer.material, wavelength);
            self.material(&layer.background, wavelength);
        }
        self.material(substrate, wavelength);
    }

    /// A structure: its stack's layers ([`Setup::stack`]) and the shapes drawn on each.
    pub fn structure(&mut self, structure: &Structure, wavelength: Wavelength) {
        let stack = structure.stack();
        self.stack(stack, wavelength);
        for layer in stack.layers().iter().rev() {
            for shape in structure.shapes(&layer.name) {
                self.shape(&layer.name, &describe(shape));
            }
        }
    }

    /// A device at `values`: its kind, every parameter's value, its stack and its shapes.
    pub fn device(&mut self, device: &dyn Device, values: &Values, wavelength: Wavelength) {
        self.parameters(device, values);
        let geometry = device.geometry(values).expect("the device's own values");
        self.structure(&geometry.structure, wavelength);
    }

    /// A device's kind and every parameter's value at `values`, without its stack.
    pub fn parameters(&mut self, device: &dyn Device, values: &Values) {
        let parameters: Vec<String> = device
            .parameters()
            .iter()
            .zip(values.as_slice())
            .map(|(p, v)| {
                if p.unit.is_empty() {
                    format!("{} {}", p.name, number(*v))
                } else {
                    format!("{} {} {}", p.name, number(*v), p.unit)
                }
            })
            .collect();
        self.item(device.kind(), &parameters.join(", "));
    }

    /// A circuit: no layers, its instances, each with its kind and every parameter's value
    /// (`values` in the order of the circuit's parameters, as its solve takes them), and the
    /// ports it exposes.
    pub fn circuit(&mut self, circuit: &Circuit, values: &[f64]) {
        let all = Component::parameters(circuit);
        for instance in circuit.netlist().instances() {
            let prefix = format!("{}.", instance.name);
            let parameters: Vec<String> = all
                .iter()
                .zip(values)
                .filter_map(|(p, v)| {
                    let name = p.name.strip_prefix(&prefix)?;
                    Some(if p.unit.is_empty() {
                        format!("{name} {}", number(*v))
                    } else {
                        format!("{name} {} {}", number(*v), p.unit)
                    })
                })
                .collect();
            self.item(
                &format!("instance {}", instance.name),
                &format!(
                    "a {}{}{}",
                    instance.component.kind(),
                    if parameters.is_empty() { "" } else { ": " },
                    parameters.join(", ")
                ),
            );
        }
        let ports: Vec<&str> = Component::ports(circuit)
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        self.item("ports", &ports.join(", "));
    }

    /// A shape on `layer`, described with its position and size and their units.
    pub fn shape(&mut self, layer: &str, what: &str) {
        self.last().shapes.push(format!("on {layer}: {what}"));
    }

    /// A setting: the wavelength, the window, the grid, the solver, the tolerance, …
    pub fn item(&mut self, what: &str, value: &str) {
        self.last().items.push((what.to_owned(), value.to_owned()));
    }

    /// The wavelength the example computes at.
    pub fn wavelength(&mut self, wavelength: Wavelength) {
        self.item("wavelength", &format!("{} µm", number(wavelength.to_um())));
    }

    /// A material: its model with its parameters, its range, its source and its index at
    /// `wavelength` (none when the wavelength is outside its range). Printed once per name.
    pub fn material(&mut self, material: &Material, wavelength: Wavelength) {
        if self.named.iter().any(|n| n == material.name()) {
            return;
        }
        self.named.push(material.name().to_owned());
        let (lo, hi) = material.range();
        let range = if hi.to_um() > 1e300 {
            "every wavelength".to_owned()
        } else {
            format!("{} to {} µm", number(lo.to_um()), number(hi.to_um()))
        };
        let p = material.provenance();
        let mut text = format!(
            "{}: {}; valid {range}",
            material.name(),
            model(material.model())
        );
        let mut source = p.reference.clone();
        if !p.doi.is_empty() {
            source += &format!(", doi:{}", p.doi);
        }
        if let Some(t) = p.temperature {
            source += &format!(", at {} K", number(t));
        }
        text += &format!("\n      source: {source}");
        if !p.notes.is_empty() {
            text += &format!("\n      {}", p.notes);
        }
        text += &format!(
            "\n      {} at {} µm",
            index_at(material, wavelength),
            number(wavelength.to_um())
        );
        self.materials.push(text);
    }

    /// A material given by its index alone, not a [`Material`]: what it is and where the
    /// number comes from.
    pub fn medium(&mut self, name: &str, what: &str) {
        if self.named.iter().any(|n| n == name) {
            return;
        }
        self.named.push(name.to_owned());
        self.materials.push(format!("{name}: {what}"));
    }

    /// Prints the setup, then a blank line. With `PHOTONOXIDE_SETUP_ONLY` set (to anything but
    /// `0`), stops the example there, successfully.
    pub fn print(&self) {
        println!("Setup");
        if self.parts.iter().any(|p| !p.layers.is_empty()) {
            println!(
                "  (layers from the top down: the bottom one, the substrate, is the first laid down)"
            );
        }
        for part in &self.parts {
            let indent = if part.title.is_empty() {
                "  "
            } else {
                println!("  {}:", part.title);
                "    "
            };
            if !part.layers.is_empty() {
                println!("{indent}layers:");
                for l in &part.layers {
                    println!("{indent}  {l}");
                }
            }
            if !part.shapes.is_empty() {
                println!("{indent}shapes:");
                for s in &part.shapes {
                    println!("{indent}  {s}");
                }
            }
            for (what, value) in &part.items {
                println!("{indent}{what}: {value}");
            }
        }
        if !self.materials.is_empty() {
            println!("  materials:");
            for m in &self.materials {
                println!("    {m}");
            }
        }
        println!();
        if std::env::var(SETUP_ONLY).is_ok_and(|v| !v.is_empty() && v != "0") {
            std::process::exit(0);
        }
    }
}

/// A refractive index, for [`Setup::layer`]: `n 3.476` (six decimals).
pub fn n(index: f64) -> String {
    format!("n {index:.6}")
}

/// A relative permittivity, for [`Setup::layer`]: `ε 12`.
pub fn eps(permittivity: f64) -> String {
    format!("ε {}", number(permittivity))
}

/// A number to 7 significant digits, as short as it reads: `0.6961663`, `214.2857`, `2.5`.
pub fn number(x: f64) -> String {
    let r: f64 = format!("{x:.6e}").parse().unwrap_or(x);
    if r == 0.0 {
        "0".to_owned()
    } else if r.abs() >= 1e7 || r.abs() < 1e-4 {
        format!("{r:e}")
    } else {
        format!("{r}")
    }
}

/// A length in µm, in nm below a micrometre: `220 nm`, `2 µm`.
pub fn length(um: f64) -> String {
    if um != 0.0 && um.abs() < 1.0 {
        format!("{} nm", number(um * 1000.0))
    } else {
        format!("{} µm", number(um))
    }
}

/// A point in µm: `(0, 0.35) µm`.
pub fn point(x_um: f64, y_um: f64) -> String {
    format!("({}, {}) µm", number(x_um), number(y_um))
}

/// `material`'s index at `wavelength`, n or n + iκ, or why there is none.
fn index_at(material: &Material, wavelength: Wavelength) -> String {
    match material.refractive_index(wavelength) {
        Ok(n) if n.im == 0.0 => format!("n {:.6}", n.re),
        Ok(n) => format!("n {:.6} + {:.3e}i", n.re, n.im),
        Err(_) => "outside its range".to_owned(),
    }
}

/// A shape, with its position and size.
pub fn describe(shape: &Shape) -> String {
    match shape {
        Shape::Rect {
            center,
            width,
            height,
        } => format!(
            "rectangle centred at {}, {} along x by {} along y",
            point(center.x.to_um(), center.y.to_um()),
            length(width.to_um()),
            length(height.to_um())
        ),
        Shape::Circle { center, radius } => format!(
            "disk centred at {}, radius {}",
            point(center.x.to_um(), center.y.to_um()),
            length(radius.to_um())
        ),
        Shape::Ring {
            center,
            inner,
            outer,
        } => format!(
            "ring centred at {}, radii {} to {}",
            point(center.x.to_um(), center.y.to_um()),
            length(inner.to_um()),
            length(outer.to_um())
        ),
        Shape::Polygon(p) => format!(
            "polygon of {} vertices, {}",
            p.vertices().len(),
            span(shape)
        ),
        _ => format!("shape, {}", span(shape)),
    }
}

/// Where a shape lies: its bounding box, from corner to corner, and its size.
fn span(shape: &Shape) -> String {
    let b = shape.bounds();
    let (x0, y0, x1, y1) = (
        b.min.x.to_um(),
        b.min.y.to_um(),
        b.max.x.to_um(),
        b.max.y.to_um(),
    );
    format!(
        "within {} to {}: {} along x by {} along y",
        point(x0, y0),
        point(x1, y1),
        length(x1 - x0),
        length(y1 - y0)
    )
}

/// A list of numbers: `0.696166, 0.407943`.
fn numbers(xs: impl IntoIterator<Item = f64>) -> String {
    xs.into_iter().map(number).collect::<Vec<_>>().join(", ")
}

/// A dispersion model with its parameters.
fn model(m: &Model) -> String {
    match m {
        Model::Constant(n) if n.im == 0.0 => format!("a constant index, n {}", number(n.re)),
        Model::Constant(n) => format!("a constant index, n {} + {}i", number(n.re), number(n.im)),
        Model::Sellmeier { a, terms } => format!(
            "Sellmeier, n² = A + Σ Bλ²/(λ² − C²), A = {}, B = {}, C = {} µm",
            number(*a),
            numbers(terms.iter().map(|t| t.b)),
            numbers(terms.iter().map(|t| t.c))
        ),
        Model::Cauchy(a) => format!(
            "Cauchy, n = A₀ + A₁/λ² + A₂/λ⁴ + …, A = {} (λ in µm)",
            numbers(a.iter().copied())
        ),
        Model::Drude {
            eps_inf,
            plasma,
            damping,
        } => format!(
            "Drude, ε = ε∞ − ω_p²/(ω² + iγω), ε∞ = {}, f_p = {} c/µm, γ/2π = {} c/µm",
            number(*eps_inf),
            number(plasma.to_natural()),
            number(*damping)
        ),
        Model::Lorentz { eps_inf, poles } => format!(
            "Lorentz, ε = ε∞ + Σ Δε ω₀²/(ω₀² − ω² − iγω), ε∞ = {}, (Δε, f₀ c/µm, γ/2π c/µm) = {}",
            number(*eps_inf),
            poles
                .iter()
                .map(|p| format!(
                    "({}, {}, {})",
                    number(p.strength),
                    number(p.resonance.to_natural()),
                    number(p.damping)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Model::Tabulated(t) => format!(
            "a table of {} wavelengths, {} to {} µm, between them natural cubic splines",
            t.wavelengths().len(),
            number(t.wavelengths()[0]),
            number(t.wavelengths()[t.wavelengths().len() - 1])
        ),
        Model::Formula {
            number: k,
            coefficients,
        } => format!(
            "refractiveindex.info's formula {k}, C = {} (λ in µm)",
            numbers(coefficients.iter().copied())
        ),
        Model::WithExtinction { n, k } => format!(
            "{}, with a measured κ at {} wavelengths",
            model(n),
            k.wavelengths().len()
        ),
        Model::Pikhtin {
            a,
            eps2,
            g3,
            energies,
        } => format!(
            "Pikhtin and Yas'kov's form (Skauli et al. 2003, Eq. 12), A = {}, ⟨ε₂⟩ = {}, G₃ = {} eV², \
             E₀…E₃ = {} eV",
            number(*a),
            number(*eps2),
            number(*g3),
            numbers(energies.iter().copied())
        ),
        Model::Afromowitz { e0, ed, gap } => format!(
            "Afromowitz's single-oscillator form, E₀ = {} eV, E_d = {} eV, E_Γ = {} eV",
            number(*e0),
            number(*ed),
            number(*gap)
        ),
        Model::Oriented {
            principal,
            orientation,
        } => format!(
            "a crystal, along its x: {}; y: {}; z: {}; {}",
            model(&principal[0]),
            model(&principal[1]),
            model(&principal[2]),
            match orientation {
                Orientation::Cut { normal, along_x } => format!(
                    "{}-cut, its {} axis along the chip's x",
                    normal.name(),
                    along_x.name()
                ),
                Orientation::Euler {
                    phi_deg,
                    theta_deg,
                    psi_deg,
                } => format!(
                    "Euler angles (z-x-z) {}°, {}°, {}°",
                    number(*phi_deg),
                    number(*theta_deg),
                    number(*psi_deg)
                ),
                _ => "turned onto the chip".to_owned(),
            }
        ),
        _ => "a dispersion model".to_owned(),
    }
}

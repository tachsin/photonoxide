//! Layer stacks: the vertical structure of a chip, and what is drawn on its layers.
//!
//! A [`LayerStack`] is a substrate, layers from bottom to top, and a cladding above. Each layer
//! has two materials: its `material` where the layout draws shapes, and its `background`
//! everywhere else (where the layer was etched away and refilled, usually by the cladding).
//! A [`Structure`] is a stack plus the shapes drawn on its layers, and answers which material
//! fills any point (x, y, z), the question a solver's grid asks.
//!
//! z = 0 is the bottom of the first layer; the substrate is below it, the cladding above the
//! last layer.

use std::sync::OnceLock;

use crate::geometry::{Index, Point, Shape};
use crate::material::{self, Material};
use crate::units::Length;
use crate::{Error, Result};

/// A layer: its thickness, the material of what is drawn, and the material elsewhere.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    /// Its name, e.g. `"Si"`.
    pub name: String,
    /// Its thickness.
    pub thickness: Length,
    /// The material where the layout draws shapes on it.
    pub material: Material,
    /// The material everywhere else in the layer.
    pub background: Material,
}

/// A substrate, layers from bottom to top, and a cladding. Build it with [`LayerStack::new`]
/// or a preset such as [`LayerStack::soi`].
#[derive(Clone, Debug, PartialEq)]
pub struct LayerStack {
    substrate: Material,
    layers: Vec<Layer>,
    cladding: Material,
}

impl LayerStack {
    /// A stack of `layers`, bottom to top, between a substrate and a cladding.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] with no layers, a thickness that isn't positive and finite, or
    /// two layers with the same name.
    pub fn new(substrate: Material, layers: Vec<Layer>, cladding: Material) -> Result<LayerStack> {
        if layers.is_empty() {
            return Err(Error::invalid("layer stack", "needs at least one layer"));
        }
        for (i, layer) in layers.iter().enumerate() {
            let t = layer.thickness.to_um();
            if !t.is_finite() || t <= 0.0 {
                return Err(Error::invalid(
                    "layer stack",
                    format!("layer {} has thickness {}", layer.name, layer.thickness),
                ));
            }
            if layers[..i].iter().any(|l| l.name == layer.name) {
                return Err(Error::invalid(
                    "layer stack",
                    format!("two layers are named {}", layer.name),
                ));
            }
        }
        Ok(LayerStack {
            substrate,
            layers,
            cladding,
        })
    }

    /// Silicon-on-insulator: a silicon substrate, `buried_oxide` of silica, a silicon device
    /// layer `device` thick (named `"Si"`), etched regions refilled with silica, and a silica
    /// cladding.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both thicknesses are positive and finite.
    pub fn soi(device: Length, buried_oxide: Length) -> Result<LayerStack> {
        LayerStack::new(
            material::silicon(),
            vec![
                Layer {
                    name: "BOX".into(),
                    thickness: buried_oxide,
                    material: material::silica(),
                    background: material::silica(),
                },
                Layer {
                    name: "Si".into(),
                    thickness: device,
                    material: material::silicon(),
                    background: material::silica(),
                },
            ],
            material::silica(),
        )
    }

    /// The standard silicon photonics platform: 220 nm of silicon on 2 µm of buried oxide,
    /// with an oxide cladding. L. Chrostowski, M. Hochberg, *Silicon Photonics Design*,
    /// Cambridge University Press (2015),
    /// [doi:10.1017/CBO9781316084168](https://doi.org/10.1017/CBO9781316084168): "The typical
    /// 200 mm (8") wafer consists of a 725 µm silicon substrate, 2 µm of oxide (buried oxide,
    /// or BOX), and 220 nm of crystalline silicon" (chapter 3, p. 49, Fig. 3.1); a 2 µm BOX
    /// couples well at both 1310 and 1550 nm and "is a common standard among silicon photonics
    /// foundries" (chapter 5). The substrate is modelled as infinitely thick. A PDK sets its
    /// own thicknesses.
    pub fn soi_220() -> LayerStack {
        LayerStack {
            substrate: material::silicon(),
            layers: vec![
                Layer {
                    name: "BOX".into(),
                    thickness: Length::um(2.0),
                    material: material::silica(),
                    background: material::silica(),
                },
                Layer {
                    name: "Si".into(),
                    thickness: Length::nm(220.0),
                    material: material::silicon(),
                    background: material::silica(),
                },
            ],
            cladding: material::silica(),
        }
    }

    /// Silicon nitride on oxide: a silicon substrate, `bottom_oxide` of silica, a nitride
    /// layer `core` thick (named `"SiN"`), etched regions refilled with silica, and a silica
    /// cladding.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] unless both thicknesses are positive and finite.
    pub fn nitride(core: Length, bottom_oxide: Length) -> Result<LayerStack> {
        LayerStack::new(
            material::silicon(),
            vec![
                Layer {
                    name: "BOX".into(),
                    thickness: bottom_oxide,
                    material: material::silica(),
                    background: material::silica(),
                },
                Layer {
                    name: "SiN".into(),
                    thickness: core,
                    material: material::silicon_nitride(),
                    background: material::silica(),
                },
            ],
            material::silica(),
        )
    }

    /// The layers, bottom to top.
    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// The substrate, below the first layer.
    pub fn substrate(&self) -> &Material {
        &self.substrate
    }

    /// The cladding, above the last layer.
    pub fn cladding(&self) -> &Material {
        &self.cladding
    }

    /// The layer named `name`, with its bottom and top z.
    pub fn layer(&self, name: &str) -> Option<(&Layer, Length, Length)> {
        let mut z = Length::ZERO;
        for layer in &self.layers {
            let top = z + layer.thickness;
            if layer.name == name {
                return Some((layer, z, top));
            }
            z = top;
        }
        None
    }

    /// The total thickness of the layers.
    pub fn thickness(&self) -> Length {
        self.layers
            .iter()
            .fold(Length::ZERO, |sum, l| sum + l.thickness)
    }

    /// Where z falls: below the stack, in a layer (a layer's bottom belongs to it, its top to
    /// the one above), or above the stack.
    fn locate(&self, z: Length) -> Place {
        if z < Length::ZERO {
            return Place::Below;
        }
        let mut top = Length::ZERO;
        for (i, layer) in self.layers.iter().enumerate() {
            top += layer.thickness;
            if z < top {
                return Place::In(i);
            }
        }
        Place::Above
    }
}

/// Where a height falls in a stack.
enum Place {
    Below,
    In(usize),
    Above,
}

/// A layer stack and the shapes drawn on its layers.
#[derive(Clone, Debug, PartialEq)]
pub struct Structure {
    stack: LayerStack,
    /// for each layer, the shapes drawn on it
    shapes: Vec<Vec<Shape>>,
    /// for each layer, an index over its shapes, made when first asked for
    index: Indexes,
}

/// The layers' indexes, made once on first use; equal whether made or not, since they follow
/// from the shapes.
#[derive(Clone, Debug, Default)]
struct Indexes(OnceLock<Vec<Index>>);

impl PartialEq for Indexes {
    fn eq(&self, _: &Indexes) -> bool {
        true
    }
}

/// A layer with more shapes than this is searched through its [`Index`]; with fewer, one by one.
const INDEXED: usize = 8;

impl Structure {
    /// A stack with nothing drawn yet.
    pub fn new(stack: LayerStack) -> Structure {
        let shapes = vec![Vec::new(); stack.layers.len()];
        Structure {
            stack,
            shapes,
            index: Indexes::default(),
        }
    }

    /// Draws `shape` on the layer named `layer`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the stack has no such layer.
    pub fn draw(&mut self, layer: &str, shape: Shape) -> Result<&mut Structure> {
        let i = self
            .stack
            .layers
            .iter()
            .position(|l| l.name == layer)
            .ok_or_else(|| {
                Error::invalid("structure", format!("the stack has no layer {layer}"))
            })?;
        self.shapes[i].push(shape);
        self.index = Indexes::default();
        Ok(self)
    }

    /// The index over the shapes drawn on the layer named `layer`, for finding those near a
    /// point or a cell ([`Index::meeting`]); `None` for an unknown layer.
    pub fn index(&self, layer: &str) -> Option<&Index> {
        let i = self.stack.layers.iter().position(|l| l.name == layer)?;
        Some(&self.indexes()[i])
    }

    fn indexes(&self) -> &[Index] {
        self.index
            .0
            .get_or_init(|| self.shapes.iter().map(|s| Index::of(s)).collect())
    }

    /// Whether a shape drawn on layer `i` contains `p`: the shapes whose boxes hold it, through
    /// the layer's index when it has many. The answer is the one testing every shape gives.
    fn covered(&self, i: usize, p: Point) -> bool {
        let shapes = &self.shapes[i];
        if shapes.len() <= INDEXED {
            return shapes.iter().any(|s| s.contains(p));
        }
        self.indexes()[i].any_at(p, |k| shapes[k].contains(p))
    }

    /// The stack.
    pub fn stack(&self) -> &LayerStack {
        &self.stack
    }

    /// The shapes drawn on the layer named `layer` (empty for an unknown name).
    pub fn shapes(&self, layer: &str) -> &[Shape] {
        self.stack
            .layers
            .iter()
            .position(|l| l.name == layer)
            .map_or(&[], |i| &self.shapes[i])
    }

    /// The material at (x, y, z): the substrate below the stack, the cladding above it, and
    /// within a layer its material inside a drawn shape and its background elsewhere.
    pub fn material_at(&self, p: Point, z: Length) -> &Material {
        match self.stack.locate(z) {
            Place::Below => &self.stack.substrate,
            Place::Above => &self.stack.cladding,
            Place::In(i) => {
                let layer = &self.stack.layers[i];
                if self.covered(i, p) {
                    &layer.material
                } else {
                    &layer.background
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soi_220_is_220_nm_of_silicon_on_2_um_of_oxide() {
        let stack = LayerStack::soi_220();
        let (si, bottom, top) = stack.layer("Si").unwrap();
        assert_eq!(si.material.name(), "Si");
        assert_eq!(si.background.name(), "SiO2");
        assert!((bottom.to_um() - 2.0).abs() < 1e-12);
        assert!(((top - bottom).to_nm() - 220.0).abs() < 1e-9);
        assert_eq!(stack.cladding().name(), "SiO2");
        assert_eq!(stack.substrate().name(), "Si");
        assert!((stack.thickness().to_um() - 2.22).abs() < 1e-12);
        // the preset is what the general constructor builds
        assert_eq!(
            stack,
            LayerStack::soi(Length::nm(220.0), Length::um(2.0)).unwrap()
        );
    }

    #[test]
    fn a_waveguide_is_silicon_inside_and_oxide_around() {
        let mut s = Structure::new(LayerStack::soi_220());
        let wg = Shape::rect(Point::um(0.0, 0.0), Length::um(10.0), Length::nm(500.0)).unwrap();
        s.draw("Si", wg).unwrap();
        let mid = Length::um(2.11);
        assert_eq!(s.material_at(Point::um(0.0, 0.0), mid).name(), "Si");
        assert_eq!(s.material_at(Point::um(0.0, 0.3), mid).name(), "SiO2");
        // above the device layer: cladding; in the buried oxide: oxide; below: the substrate
        assert_eq!(
            s.material_at(Point::um(0.0, 0.0), Length::um(2.3)).name(),
            "SiO2"
        );
        assert_eq!(
            s.material_at(Point::um(0.0, 0.0), Length::um(1.0)).name(),
            "SiO2"
        );
        assert_eq!(
            s.material_at(Point::um(0.0, 0.0), Length::um(-0.1)).name(),
            "Si"
        );
        assert_eq!(s.shapes("Si").len(), 1);
        assert!(s.shapes("BOX").is_empty());
    }

    #[test]
    fn drawing_on_an_unknown_layer_is_an_error() {
        let mut s = Structure::new(LayerStack::soi_220());
        let shape = Shape::circle(Point::um(0.0, 0.0), Length::um(1.0)).unwrap();
        let e = s.draw("Metal", shape).unwrap_err();
        assert!(e.to_string().contains("Metal"), "{e}");
    }

    #[test]
    fn bad_stacks_are_errors() {
        assert!(LayerStack::soi(Length::ZERO, Length::um(2.0)).is_err());
        assert!(LayerStack::nitride(Length::nm(300.0), Length::um(-1.0)).is_err());
        assert!(LayerStack::new(material::silicon(), Vec::new(), material::silica()).is_err());
        let layer = Layer {
            name: "a".into(),
            thickness: Length::um(1.0),
            material: material::silica(),
            background: material::silica(),
        };
        assert!(
            LayerStack::new(
                material::silicon(),
                vec![layer.clone(), layer],
                material::silica()
            )
            .is_err()
        );
    }

    #[test]
    fn a_nitride_stack_has_a_nitride_core() {
        let stack = LayerStack::nitride(Length::nm(300.0), Length::um(3.0)).unwrap();
        let (sin, bottom, _) = stack.layer("SiN").unwrap();
        assert_eq!(sin.material.name(), "Si3N4");
        assert!((bottom.to_um() - 3.0).abs() < 1e-12);
    }
}

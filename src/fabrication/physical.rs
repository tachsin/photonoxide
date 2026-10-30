//! A design as it is made: its structure, and the film physically there in each place. The
//! optical structure sometimes draws one material in place of another (a heater's metal drawn
//! with the oxide's optics, since the mode is solved without it); the physical design names the
//! film that is there, so a flow deposits it.

use crate::material::Material;
use crate::stack::Structure;
use crate::{Error, Result};

use super::stated::Source;

/// A film: what is physically there, and the material the optical structure draws for it (its
/// own, or the one standing in for it).
#[derive(Clone, Debug, PartialEq)]
pub struct Film {
    /// What is there, e.g. `"Si"`, `"SiO2"`, `"LiNbO3"`, `"Cu/Au"`, `"TiN"`.
    pub name: String,
    /// The material the optical structure draws.
    pub optical: Material,
}

impl Film {
    /// The film a catalogue material is: its own name, and itself.
    pub fn of(material: &Material) -> Film {
        Film {
            name: film_name(material.name()),
            optical: material.clone(),
        }
    }
}

/// The film a material of the catalogue is: silicon, silica, lithium niobate (any of its axes'
/// or cuts' materials), or the material's own name.
pub fn film_name(material: &str) -> String {
    let lower = material.to_lowercase();
    if lower.contains("linbo3") || lower.contains("linbo₃") || lower.starts_with("ln") {
        "LiNbO3".into()
    } else if lower == "sio2" || lower == "sio₂" || lower.starts_with("sio2 ") {
        "SiO2".into()
    } else if lower == "si" || lower.starts_with("si (") || lower.starts_with("si ") {
        "Si".into()
    } else {
        material.into()
    }
}

/// Where the optical structure draws another material in place of the film that is there.
#[derive(Clone, Debug, PartialEq)]
pub struct StandIn {
    /// The layer.
    pub layer: String,
    /// The film that is there.
    pub film: String,
    /// What the optical structure draws in its place.
    pub drawn_as: String,
    /// Why, in plain words.
    pub why: String,
    /// The source that says which film it is.
    pub source: Source,
}

/// A design as it is made: its structure, and for the substrate, each layer (its drawn shapes'
/// film and its background's) and the cladding, the film that is there.
#[derive(Clone, Debug, PartialEq)]
pub struct Physical {
    structure: Structure,
    substrate: Film,
    layers: Vec<(Film, Film)>,
    cladding: Film,
    stand_ins: Vec<StandIn>,
}

impl Physical {
    /// A structure whose every material is the film that is there.
    pub fn of(structure: Structure) -> Physical {
        let stack = structure.stack();
        let substrate = Film::of(stack.substrate());
        let cladding = Film::of(stack.cladding());
        let layers = stack
            .layers()
            .iter()
            .map(|l| (Film::of(&l.material), Film::of(&l.background)))
            .collect();
        Physical {
            structure,
            substrate,
            layers,
            cladding,
            stand_ins: Vec::new(),
        }
    }

    /// The same design, its layer `layer`'s drawn shapes the film `film`, which the optical
    /// structure draws as it does: `why` says why, `source` which film it is.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidValue`] if the structure has no such layer.
    pub fn with_stand_in(
        mut self,
        layer: &str,
        film: &str,
        why: &str,
        source: Source,
    ) -> Result<Physical> {
        let i = self
            .structure
            .stack()
            .layers()
            .iter()
            .position(|l| l.name == layer)
            .ok_or_else(|| {
                Error::invalid("fabrication", format!("the stack has no layer {layer}"))
            })?;
        let drawn_as = self.layers[i].0.name.clone();
        self.layers[i].0.name = film.into();
        self.stand_ins.push(StandIn {
            layer: layer.into(),
            film: film.into(),
            drawn_as,
            why: why.into(),
            source,
        });
        Ok(self)
    }

    /// Its structure, as the optical model draws it.
    pub fn structure(&self) -> &Structure {
        &self.structure
    }

    /// The substrate's film.
    pub fn substrate(&self) -> &Film {
        &self.substrate
    }

    /// The cladding's film.
    pub fn cladding(&self) -> &Film {
        &self.cladding
    }

    /// For each layer, bottom to top, its drawn shapes' film and its background's.
    pub fn layers(&self) -> &[(Film, Film)] {
        &self.layers
    }

    /// Where the optical structure draws another material in place of the film there.
    pub fn stand_ins(&self) -> &[StandIn] {
        &self.stand_ins
    }

    /// The film names, substrate, each layer's (drawn, background) and cladding: what a made
    /// design must have, besides its structure.
    pub fn names(&self) -> (String, Vec<(String, String)>, String) {
        (
            self.substrate.name.clone(),
            self.layers
                .iter()
                .map(|(d, b)| (d.name.clone(), b.name.clone()))
                .collect(),
            self.cladding.name.clone(),
        )
    }
}

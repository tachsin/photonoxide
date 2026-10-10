//! Circuits from a netlist given as data: [`circuit_spectrum`].

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde_json::Value;

use super::{Spectrum, parse_convention};
use crate::circuit::components::{
    AddDropRing, AllPassRing, Bend, Coupler, DirectionalCoupler, Dispersion, PhaseShifter,
    Supermodes, Waveguide, YBranch,
};
use crate::circuit::{Component, Fixed, Netlist, SMatrix};
use crate::compact::Measured;
use crate::compact::touchstone::Touchstone;
use crate::units::Wavelength;
use crate::{Error, Result};

/// A JSON object's members in the order they are written: a netlist's instances and ports
/// keep the order they were given in.
#[derive(Debug)]
struct Ordered<V>(Vec<(String, V)>);

impl<'de, V: Deserialize<'de>> Deserialize<'de> for Ordered<V> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct Members<V>(PhantomData<V>);
        impl<'de, V: Deserialize<'de>> Visitor<'de> for Members<V> {
            type Value = Ordered<V>;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object")
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Ordered<V>, A::Error> {
                let mut members = Vec::new();
                while let Some(member) = map.next_entry()? {
                    members.push(member);
                }
                Ok(Ordered(members))
            }
        }
        deserializer.deserialize_map(Members(PhantomData))
    }
}

/// A netlist as data.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Data {
    instances: Ordered<BTreeMap<String, Value>>,
    #[serde(default)]
    connections: Vec<[String; 2]>,
    ports: Ordered<String>,
}

/// An instance's settings: what builds its component, taken out of its members one by one;
/// what is left are its parameters' values.
struct Settings<'a> {
    instance: &'a str,
    kind: String,
    members: BTreeMap<String, Value>,
}

impl Settings<'_> {
    fn error(&self, reason: impl fmt::Display) -> Error {
        Error::invalid(
            "netlist",
            format!("instance {:?} ({}): {reason}", self.instance, self.kind),
        )
    }

    fn number(&mut self, name: &str) -> Result<Option<f64>> {
        match self.members.remove(name) {
            None => Ok(None),
            Some(Value::Number(n)) => Ok(n.as_f64()),
            Some(other) => Err(self.error(format!("{name} must be a number, not {other}"))),
        }
    }

    fn required(&mut self, name: &str) -> Result<f64> {
        self.number(name)?
            .ok_or_else(|| self.error(format!("needs {name}")))
    }

    fn text(&mut self, name: &str) -> Result<Option<String>> {
        match self.members.remove(name) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s)),
            Some(other) => Err(self.error(format!("{name} must be a string, not {other}"))),
        }
    }

    /// The guide's mode: `n_eff` and `n_g` at `wavelength_um`, with `dispersion_ps_per_nm_km`
    /// and, unless the loss is the component's parameter, `loss_db_per_cm`.
    fn guide(&mut self, with_loss: bool) -> Result<Dispersion> {
        let wavelength = Wavelength::um(self.required("wavelength_um")?)?;
        let mut guide = Dispersion::new(wavelength, self.required("n_eff")?, self.required("n_g")?);
        if let Some(d) = self.number("dispersion_ps_per_nm_km")? {
            guide = guide.with_dispersion(d);
        }
        if with_loss && let Some(loss) = self.number("loss_db_per_cm")? {
            guide = guide.with_loss(loss);
        }
        Ok(guide)
    }

    /// The component an instance of this kind is.
    fn build(&mut self) -> Result<Arc<dyn Component>> {
        let component: Arc<dyn Component> = match self.kind.as_str() {
            "waveguide" => Arc::new(Waveguide::new(self.guide(false)?)),
            "bend" => {
                let radius = self.required("radius_um")?;
                Arc::new(Bend::new(radius, self.guide(true)?)?)
            }
            "phase-shifter" => Arc::new(PhaseShifter::new()),
            "coupler" => match self.number("excess_loss_db")? {
                Some(db) => Arc::new(Coupler::new().with_excess_loss(db)?),
                None => Arc::new(Coupler::new()),
            },
            "directional-coupler" => {
                let coupling = self.required("coupling_per_um")?;
                let guide = self.guide(true)?;
                Arc::new(DirectionalCoupler::new(Supermodes::from_coupling(
                    guide, coupling,
                ))?)
            }
            "y-branch" => Arc::new(YBranch::new()),
            "ring-all-pass" => {
                let ring = AllPassRing::new(self.guide(true)?)?;
                match self.number("round_trip_loss_db")? {
                    Some(db) => Arc::new(ring.with_round_trip_loss(db)?),
                    None => Arc::new(ring),
                }
            }
            "ring-add-drop" => {
                let ring = AddDropRing::new(self.guide(true)?)?;
                match self.number("round_trip_loss_db")? {
                    Some(db) => Arc::new(ring.with_round_trip_loss(db)?),
                    None => Arc::new(ring),
                }
            }
            "terminator" => Arc::new(Fixed::new("terminator", &["o1"], SMatrix::zeros(1))?),
            "touchstone" => {
                let file = self
                    .text("file")?
                    .ok_or_else(|| self.error("needs its file"))?;
                let convention = self.text("convention")?.ok_or_else(|| {
                    self.error("needs its file's convention, \"physics\" or \"engineering\"")
                })?;
                Arc::new(measured(Path::new(&file), &convention)?)
            }
            other => {
                return Err(self.error(format!(
                    "no kind {other:?}; the kinds are {}",
                    KINDS.join(", ")
                )));
            }
        };
        Ok(component)
    }
}

/// The kinds of instance a netlist can have.
const KINDS: [&str; 11] = [
    "waveguide",
    "bend",
    "phase-shifter",
    "coupler",
    "directional-coupler",
    "y-branch",
    "ring-all-pass",
    "ring-add-drop",
    "terminator",
    "touchstone",
    "fixed",
];

/// A measured component: a Touchstone file's S-parameters, interpolated between its
/// wavelengths ([`Measured`]). λ = c/f loses the last bit of a wavelength written as c/λ, so
/// each is rounded to the femtometre, as the studio's chips do: a sweep from the file's first
/// wavelength then starts inside its band.
fn measured(path: &Path, convention: &str) -> Result<Measured> {
    let spectrum = Touchstone::read(path)?.spectrum(parse_convention(convention)?)?;
    let wavelengths = spectrum
        .wavelengths()
        .iter()
        .map(|w| Wavelength::um((w.to_um() * 1e9).round() / 1e9))
        .collect::<Result<Vec<_>>>()?;
    let spectrum = crate::circuit::Spectrum::new(
        spectrum.ports().to_vec(),
        wavelengths,
        spectrum.matrices().to_vec(),
    )?;
    Measured::new(&spectrum, path.display().to_string())
}

/// A fixed S-matrix (kind `"fixed"`): its `ports`, and `s` as rows of `[re, im]` pairs.
fn fixed(settings: &mut Settings<'_>) -> Result<Arc<dyn Component>> {
    let ports: Vec<String> = match settings.members.remove("ports") {
        Some(v) => serde_json::from_value(v)
            .map_err(|e| settings.error(format!("ports must be a list of names: {e}")))?,
        None => return Err(settings.error("needs its ports")),
    };
    let rows: Vec<Vec<[f64; 2]>> = match settings.members.remove("s") {
        Some(v) => serde_json::from_value(v)
            .map_err(|e| settings.error(format!("s must be rows of [re, im] pairs: {e}")))?,
        None => return Err(settings.error("needs its S-matrix, s")),
    };
    let rows = rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|[re, im]| crate::Complex64::new(re, im))
                .collect()
        })
        .collect();
    let names: Vec<&str> = ports.iter().map(String::as_str).collect();
    Ok(Arc::new(Fixed::new(
        "fixed",
        &names,
        SMatrix::from_rows(rows)?,
    )?))
}

/// The S-matrices at `wavelength_um` of the circuit `netlist` describes, as JSON:
///
/// ```json
/// {
///   "instances": {
///     "split": {"kind": "coupler", "coupling": 0.5},
///     "upper": {"kind": "waveguide", "n_eff": 2.44506, "n_g": 4.172901, "wavelength_um": 1.55,
///               "length": 150},
///     "lower": {"kind": "waveguide", "n_eff": 2.44506, "n_g": 4.172901, "wavelength_um": 1.55,
///               "length": 100},
///     "combine": {"kind": "coupler", "coupling": 0.5}
///   },
///   "connections": [["split.o3", "upper.o1"], ["split.o4", "lower.o1"],
///                   ["upper.o2", "combine.o2"], ["lower.o2", "combine.o1"]],
///   "ports": {"in1": "split.o2", "in2": "split.o1", "out1": "combine.o3", "out2": "combine.o4"}
/// }
/// ```
///
/// Each instance names its `kind`, the settings that build it, and its parameters' values
/// (those of [`crate::circuit::components`], by the names its components give them; a
/// parameter not given takes its default). The kinds and their settings:
///
/// - `"waveguide"`: its mode's `n_eff` and `n_g` at `wavelength_um`, and optionally
///   `dispersion_ps_per_nm_km` ([`Dispersion`]); parameters `length` (µm) and `loss` (dB/cm).
/// - `"bend"`: `radius_um` and the guide's settings, with `loss_db_per_cm`; parameter `angle`
///   (degrees).
/// - `"phase-shifter"`: parameter `phase` (rad).
/// - `"coupler"`: an ideal 2 × 2 coupler, optionally `excess_loss_db`; parameter `coupling` (the
///   share of power crossing, κ²).
/// - `"directional-coupler"`: `coupling_per_um` and the guide's settings; parameter `length` (µm).
/// - `"y-branch"`: parameter `excess_loss` (dB).
/// - `"ring-all-pass"` and `"ring-add-drop"`: the guide's settings, optionally
///   `round_trip_loss_db`; parameters `length` (the round trip, µm), `coupling` and, for the
///   add-drop ring, `coupling_drop`.
/// - `"terminator"`: a perfect absorber on one port, `o1`.
/// - `"touchstone"`: a measured component, the S-parameters of the Touchstone `file` in its
///   `convention` (`"physics"` or `"engineering"`), ports `o1`, `o2`, …
/// - `"fixed"`: an S-matrix the same at every wavelength, its `ports` (names) and `s` (rows of
///   `[re, im]` pairs).
///
/// Ports are written `instance.port`. `connections` joins pairs of them; `ports` names the
/// circuit's external ports, in the order the S-matrices take them. Instances and ports keep the
/// order they are written in.
///
/// # Errors
///
/// [`Error::Parse`] for text that isn't such a netlist; [`Error::InvalidValue`] for an unknown
/// kind, a missing or wrong setting, or a wavelength that isn't positive and finite;
/// [`Error::Netlist`] for an unknown parameter or port, a value out of range, a port connected
/// twice or left unconnected; and the errors of the components and of the solve.
pub fn circuit_spectrum(netlist: &str, wavelength_um: &[f64]) -> Result<Spectrum> {
    let data: Data = serde_json::from_str(netlist).map_err(|e| Error::Parse {
        what: "netlist".into(),
        reason: e.to_string(),
    })?;
    let mut net = Netlist::new();
    let mut values = Vec::new();
    for (name, mut members) in data.instances.0 {
        let kind = match members.remove("kind") {
            Some(Value::String(kind)) => kind,
            _ => {
                return Err(Error::invalid(
                    "netlist",
                    format!(
                        "instance {name:?} needs its kind, one of {}",
                        KINDS.join(", ")
                    ),
                ));
            }
        };
        let mut settings = Settings {
            instance: &name,
            kind,
            members,
        };
        let component = if settings.kind == "fixed" {
            fixed(&mut settings)?
        } else {
            settings.build()?
        };
        for (parameter, value) in &settings.members {
            let Some(value) = value.as_f64() else {
                return Err(settings.error(format!(
                    "the value of {parameter} must be a number, not {value}"
                )));
            };
            values.push((name.clone(), parameter.clone(), value));
        }
        net.add(&name, component)?;
    }
    for (instance, parameter, value) in values {
        net.set(&instance, &parameter, value)?;
    }
    for [a, b] in &data.connections {
        net.connect(a, b)?;
    }
    for (name, port) in &data.ports.0 {
        net.expose(name, port)?;
    }
    let wavelengths = wavelength_um
        .iter()
        .map(|&w| Wavelength::um(w))
        .collect::<Result<Vec<_>>>()?;
    Ok(Spectrum::of(&net.compile()?.spectrum(&wavelengths)?))
}

//! Netlists: instances of components, the connections between their ports, and the ports left
//! open to the outside.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use super::Component;
use crate::{Error, Result};

/// A port of an instance in a netlist, written `instance.port`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PortRef {
    /// The instance's name.
    pub instance: String,
    /// The port's name within its component.
    pub port: String,
}

impl PortRef {
    /// `instance.port` read as a port reference: split at the first dot.
    ///
    /// # Errors
    ///
    /// [`NetlistError::InvalidName`] (as an [`Error::Netlist`]) without a dot, or with nothing
    /// on one side of it.
    pub fn parse(text: &str) -> Result<PortRef> {
        match text.split_once('.') {
            Some((instance, port)) if !instance.is_empty() && !port.is_empty() => Ok(PortRef {
                instance: instance.into(),
                port: port.into(),
            }),
            _ => Err(NetlistError::InvalidName {
                name: text.into(),
                reason: "a port is written instance.port".into(),
            }
            .into()),
        }
    }
}

impl fmt::Display for PortRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.instance, self.port)
    }
}

/// Why a netlist is invalid.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum NetlistError {
    /// A name is empty, holds a dot or whitespace, or a port isn't written `instance.port`.
    InvalidName {
        /// The name.
        name: String,
        /// What is wrong with it.
        reason: String,
    },
    /// A component states duplicate port or parameter names, or a default outside its range.
    InvalidComponent {
        /// The instance it was added as.
        instance: String,
        /// What is wrong with it.
        reason: String,
    },
    /// Two instances have the same name.
    DuplicateInstance(String),
    /// No instance has this name.
    UnknownInstance(String),
    /// The instance's component has no port of this name.
    UnknownPort(PortRef),
    /// The instance's component has no parameter of this name.
    UnknownParameter {
        /// The instance.
        instance: String,
        /// The parameter asked for.
        parameter: String,
    },
    /// A parameter's value is not finite or outside its range.
    ValueOutOfRange {
        /// The instance.
        instance: String,
        /// The parameter.
        parameter: String,
        /// The value given.
        value: f64,
        /// The smallest value allowed.
        min: f64,
        /// The largest value allowed.
        max: f64,
    },
    /// A port connected to itself.
    SelfConnection(PortRef),
    /// A port connected or exposed more than once.
    PortUsedTwice {
        /// The port.
        port: PortRef,
        /// What it was already used for, e.g. `"connected to wg.o1"`.
        first: String,
    },
    /// Two external ports have the same name.
    DuplicateExternal(String),
    /// A port neither connected nor exposed: light leaving it would have nowhere to go.
    Dangling(PortRef),
    /// Two connected ports carry modes of different polarizations or orders.
    ModeMismatch {
        /// One port.
        a: PortRef,
        /// The other.
        b: PortRef,
    },
    /// A component returned an S-matrix that isn't one row and column per port.
    SizeMismatch {
        /// The instance.
        instance: String,
        /// Its ports.
        ports: usize,
        /// The S-matrix's size.
        size: usize,
    },
}

impl fmt::Display for NetlistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetlistError::InvalidName { name, reason } => write!(f, "\"{name}\": {reason}"),
            NetlistError::InvalidComponent { instance, reason } => {
                write!(f, "{instance}'s component: {reason}")
            }
            NetlistError::DuplicateInstance(name) => write!(f, "two instances named {name}"),
            NetlistError::UnknownInstance(name) => write!(f, "no instance named {name}"),
            NetlistError::UnknownPort(port) => {
                write!(f, "{} has no port {}", port.instance, port.port)
            }
            NetlistError::UnknownParameter {
                instance,
                parameter,
            } => write!(f, "{instance} has no parameter {parameter}"),
            NetlistError::ValueOutOfRange {
                instance,
                parameter,
                value,
                min,
                max,
            } => write!(
                f,
                "{instance}.{parameter} must be from {min} to {max}, got {value}"
            ),
            NetlistError::SelfConnection(port) => write!(f, "{port} is connected to itself"),
            NetlistError::PortUsedTwice { port, first } => {
                write!(f, "{port} is used twice: it is already {first}")
            }
            NetlistError::DuplicateExternal(name) => write!(f, "two external ports named {name}"),
            NetlistError::Dangling(port) => write!(f, "{port} is neither connected nor exposed"),
            NetlistError::ModeMismatch { a, b } => {
                write!(f, "{a} and {b} carry different modes")
            }
            NetlistError::SizeMismatch {
                instance,
                ports,
                size,
            } => write!(
                f,
                "{instance} has {ports} ports but returned a {size} x {size} S-matrix"
            ),
        }
    }
}

impl From<NetlistError> for Error {
    fn from(e: NetlistError) -> Error {
        Error::Netlist(e)
    }
}

/// A component placed in a netlist under a name, with its parameters' values.
#[derive(Clone, Debug)]
pub struct Instance {
    /// Its name, unique in the netlist.
    pub name: String,
    /// What it is.
    pub component: Arc<dyn Component>,
    /// Its parameters' values, in the order of the component's
    /// [`parameters`](Component::parameters).
    pub values: Vec<f64>,
}

impl Instance {
    /// The position of the port named `port`.
    pub fn port(&self, port: &str) -> Option<usize> {
        self.component.ports().iter().position(|p| p.name == port)
    }
}

/// A circuit's description: instances, connections and external ports.
///
/// Built a step at a time, each step checked as it is taken: [`Netlist::add`] an instance,
/// [`Netlist::connect`] two ports, [`Netlist::expose`] a port to the outside. Every port must end
/// up connected or exposed, exactly once; [`Netlist::problems`] lists what isn't yet, and
/// a netlist with problems can't be solved.
///
/// A connection joins the two ports' reference planes with nothing between them: what leaves
/// one enters the other. The external ports are the circuit's own, in the order they were
/// exposed.
#[derive(Clone, Debug, Default)]
pub struct Netlist {
    instances: Vec<Instance>,
    by_name: HashMap<String, usize>,
    connections: Vec<(PortRef, PortRef)>,
    external: Vec<(String, PortRef)>,
}

/// Checks a name: not empty, no dot, no whitespace.
fn check_name(name: &str) -> std::result::Result<(), NetlistError> {
    if name.is_empty() || name.contains('.') || name.chars().any(char::is_whitespace) {
        return Err(NetlistError::InvalidName {
            name: name.into(),
            reason: "a name must be non-empty, without dots or whitespace".into(),
        });
    }
    Ok(())
}

/// Checks a component's ports and parameters as stated.
fn check_component(instance: &str, c: &dyn Component) -> std::result::Result<(), NetlistError> {
    let invalid = |reason: String| NetlistError::InvalidComponent {
        instance: instance.into(),
        reason,
    };
    let ports = c.ports();
    for (i, p) in ports.iter().enumerate() {
        check_name(&p.name).map_err(|e| invalid(format!("port {e}")))?;
        if ports[..i].iter().any(|q| q.name == p.name) {
            return Err(invalid(format!("two ports named {}", p.name)));
        }
    }
    let parameters = c.parameters();
    for (i, p) in parameters.iter().enumerate() {
        check_name(&p.name).map_err(|e| invalid(format!("parameter {e}")))?;
        if parameters[..i].iter().any(|q| q.name == p.name) {
            return Err(invalid(format!("two parameters named {}", p.name)));
        }
        if !p.allows(p.default) {
            return Err(invalid(format!(
                "{}'s default {} isn't within its range, {} to {}",
                p.name, p.default, p.min, p.max
            )));
        }
    }
    Ok(())
}

/// Whether two ports' stated modes can be joined: the same polarization and order, when both
/// are stated.
fn modes_match(a: &super::Port, b: &super::Port) -> bool {
    match (&a.mode, &b.mode) {
        (Some(m), Some(n)) => m.polarization == n.polarization && m.order == n.order,
        _ => true,
    }
}

impl Netlist {
    /// An empty netlist.
    pub fn new() -> Netlist {
        Netlist::default()
    }

    /// Adds an instance of `component` named `name`, its parameters at their defaults.
    ///
    /// # Errors
    ///
    /// [`Error::Netlist`]: [`NetlistError::InvalidName`] for an empty name or one with a dot or
    /// whitespace, [`NetlistError::DuplicateInstance`] for a name taken, and
    /// [`NetlistError::InvalidComponent`] for a component with duplicate or invalid port or
    /// parameter names, or a default outside its range.
    pub fn add(&mut self, name: &str, component: Arc<dyn Component>) -> Result<()> {
        check_name(name)?;
        if self.by_name.contains_key(name) {
            return Err(NetlistError::DuplicateInstance(name.into()).into());
        }
        check_component(name, component.as_ref())?;
        let values = component.defaults();
        self.by_name.insert(name.into(), self.instances.len());
        self.instances.push(Instance {
            name: name.into(),
            component,
            values,
        });
        Ok(())
    }

    /// Sets parameter `parameter` of instance `instance` to `value`.
    ///
    /// # Errors
    ///
    /// [`Error::Netlist`]: [`NetlistError::UnknownInstance`], [`NetlistError::UnknownParameter`],
    /// or [`NetlistError::ValueOutOfRange`] for a value that isn't finite or is outside the
    /// parameter's range.
    pub fn set(&mut self, instance: &str, parameter: &str, value: f64) -> Result<()> {
        let i = self.index(instance)?;
        let inst = &mut self.instances[i];
        let Some(k) = inst
            .component
            .parameters()
            .iter()
            .position(|p| p.name == parameter)
        else {
            return Err(NetlistError::UnknownParameter {
                instance: instance.into(),
                parameter: parameter.into(),
            }
            .into());
        };
        let p = &inst.component.parameters()[k];
        if !p.allows(value) {
            return Err(NetlistError::ValueOutOfRange {
                instance: instance.into(),
                parameter: parameter.into(),
                value,
                min: p.min,
                max: p.max,
            }
            .into());
        }
        inst.values[k] = value;
        Ok(())
    }

    /// Connects port `a` to port `b`, each written `instance.port`.
    ///
    /// # Errors
    ///
    /// [`Error::Netlist`]: [`NetlistError::InvalidName`] for a port not written
    /// `instance.port`, [`NetlistError::UnknownInstance`] or [`NetlistError::UnknownPort`] for
    /// one that doesn't exist, [`NetlistError::SelfConnection`],
    /// [`NetlistError::PortUsedTwice`] for a port already connected or exposed, and
    /// [`NetlistError::ModeMismatch`] for ports whose stated modes differ.
    pub fn connect(&mut self, a: &str, b: &str) -> Result<()> {
        let (a, b) = (PortRef::parse(a)?, PortRef::parse(b)?);
        let pa = self.resolve(&a)?;
        let pb = self.resolve(&b)?;
        if a == b {
            return Err(NetlistError::SelfConnection(a).into());
        }
        self.unused(&a)?;
        self.unused(&b)?;
        if !modes_match(self.port_of(pa), self.port_of(pb)) {
            return Err(NetlistError::ModeMismatch { a, b }.into());
        }
        self.connections.push((a, b));
        Ok(())
    }

    /// Exposes `port`, written `instance.port`, as the circuit's external port `name`.
    ///
    /// # Errors
    ///
    /// [`Error::Netlist`]: [`NetlistError::InvalidName`] for an invalid name or a port not
    /// written `instance.port`, [`NetlistError::DuplicateExternal`] for a name taken,
    /// [`NetlistError::UnknownInstance`] or [`NetlistError::UnknownPort`] for a port that
    /// doesn't exist, and [`NetlistError::PortUsedTwice`] for one already connected or exposed.
    pub fn expose(&mut self, name: &str, port: &str) -> Result<()> {
        check_name(name)?;
        let p = PortRef::parse(port)?;
        if self.external.iter().any(|(n, _)| n == name) {
            return Err(NetlistError::DuplicateExternal(name.into()).into());
        }
        self.resolve(&p)?;
        self.unused(&p)?;
        self.external.push((name.into(), p));
        Ok(())
    }

    /// The instances, in the order they were added.
    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    /// The instance named `name`.
    pub fn instance(&self, name: &str) -> Option<&Instance> {
        self.by_name.get(name).map(|&i| &self.instances[i])
    }

    /// The connections, in the order they were made.
    pub fn connections(&self) -> &[(PortRef, PortRef)] {
        &self.connections
    }

    /// The external ports, each a name and the port it exposes, in the circuit's order.
    pub fn external(&self) -> &[(String, PortRef)] {
        &self.external
    }

    /// Everything that keeps the netlist from being compiled: every port neither connected nor
    /// exposed ([`NetlistError::Dangling`]), in the order of the instances and their ports. Empty
    /// when it is valid.
    pub fn problems(&self) -> Vec<NetlistError> {
        let mut used: HashMap<&PortRef, ()> = HashMap::new();
        for (a, b) in &self.connections {
            used.insert(a, ());
            used.insert(b, ());
        }
        for (_, p) in &self.external {
            used.insert(p, ());
        }
        let mut problems = Vec::new();
        for inst in &self.instances {
            for port in inst.component.ports() {
                let r = PortRef {
                    instance: inst.name.clone(),
                    port: port.name.clone(),
                };
                if !used.contains_key(&r) {
                    problems.push(NetlistError::Dangling(r));
                }
            }
        }
        problems
    }

    /// Checks that the netlist can be compiled.
    ///
    /// # Errors
    ///
    /// [`Error::Netlist`] with the first of [`Netlist::problems`].
    pub fn validate(&self) -> Result<()> {
        match self.problems().into_iter().next() {
            Some(p) => Err(p.into()),
            None => Ok(()),
        }
    }

    /// The position of instance `name`.
    pub(super) fn index(&self, name: &str) -> std::result::Result<usize, NetlistError> {
        self.by_name
            .get(name)
            .copied()
            .ok_or_else(|| NetlistError::UnknownInstance(name.into()))
    }

    /// The instance and port positions of `r`.
    pub(super) fn resolve(&self, r: &PortRef) -> std::result::Result<(usize, usize), NetlistError> {
        let i = self.index(&r.instance)?;
        let p = self.instances[i]
            .port(&r.port)
            .ok_or_else(|| NetlistError::UnknownPort(r.clone()))?;
        Ok((i, p))
    }

    fn port_of(&self, (i, p): (usize, usize)) -> &super::Port {
        &self.instances[i].component.ports()[p]
    }

    /// Fails if `r` is already connected or exposed.
    fn unused(&self, r: &PortRef) -> std::result::Result<(), NetlistError> {
        let used = |first: String| NetlistError::PortUsedTwice {
            port: r.clone(),
            first,
        };
        for (a, b) in &self.connections {
            if a == r {
                return Err(used(format!("connected to {b}")));
            }
            if b == r {
                return Err(used(format!("connected to {a}")));
            }
        }
        if let Some((name, _)) = self.external.iter().find(|(_, p)| p == r) {
            return Err(used(format!("exposed as {name}")));
        }
        Ok(())
    }
}

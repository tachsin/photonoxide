//! Expressions: a small, safe language for the numbers of a structure, with units, named
//! parameters and derivatives. It is photonoxide's own and evaluates arithmetic only: no
//! names but its own functions, the coordinates and the parameters it is given, no loops, no
//! access to anything outside.
//!
//! ```text
//! 500 nm          w/2 + gap        2*pi*x/period      if(i < 3, 0.2 um, r0)
//! 90 deg          sqrt(w^2 + h^2)  r0*(1 + 0.1*cos(4*t*2*pi))
//! ```
//!
//! - **Numbers with units:** `nm`, `um` (or `µm`), `mm` for lengths, kept in µm; `deg` and
//!   `rad` for angles, kept in radians. A unit follows a number (`500 nm`); alone, it is the
//!   unit itself (`w/nm` is w in nanometres).
//! - **Operators:** `+ - * / ^` (`^` binds tighter than a leading minus and to the right:
//!   `-2^2` is −4, `2^3^2` is 512), comparisons `< <= > >= == !=` (1 or 0), parentheses.
//! - **Functions:** `sin cos tan asin acos atan atan2 exp ln log10 sqrt abs min max clamp floor
//!   ceil round erf tanh sinc smoothstep`, `if(c, a, b)` (b where c is 0), and
//!   `piecewise(c1, a1, c2, a2, …, otherwise)`, the first a whose c isn't 0. `pi` is π;
//!   `sinc(x)` is sin(x)/x; `smoothstep(e0, e1, x)` is 3u² − 2u³ for u = (x − e0)/(e1 − e0)
//!   clamped to [0, 1]; `clamp(x, lo, hi)`.
//! - **Coordinates:** `x`, `y`, `z` and the arc length `s` (lengths), a curve's parameter `t`
//!   and an array's indices `i`, `j` (numbers), for the definitions that supply them.
//! - **Parameters:** names of a [`Parameters`] table, each an expression of the others.
//!
//! **Units are checked.** Every expression has a [`Dim`]ension: a power of length and of angle.
//! Sums and comparisons need the same length power (adding a length to an angle is refused);
//! products add powers; `sin`, `cos` and `tan` take an angle or a number, `asin`, `acos`,
//! `atan` and `atan2` give an angle; `sqrt` halves powers, and a power of a dimensioned base
//! needs a constant exponent. An angle and a number add (radians are numbers). A bare `0` fits
//! any dimension. Errors name the column (from 1) where they are.
//!
//! **Compiled once.** An expression is parsed and checked once, its constant parts folded, and
//! compiled to a flat list of instructions for a stack of at most 64 values, evaluated without
//! allocating: [`Expr::eval`]; [`Expr::try_eval`] says where a division by zero or a value
//! outside a function's domain happened; [`Expr::eval_many`] evaluates at many points at once,
//! each instruction over a block of them.
//!
//! **Derivatives** by forward mode ([`Expr::eval_dual`]): every value carries its derivative
//! along one direction of the parameters (dual numbers, R. E. Wengert, Commun. ACM 7, 463
//! (1964), doi:10.1145/355586.364791), so a shape's boundary can be differentiated with
//! respect to a parameter.

use std::fmt;

mod parse;
mod special;
#[cfg(test)]
mod tests;

pub use parse::Expr;

/// A dimension: a power of length and a power of angle. Lengths are in µm and angles in
/// radians.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Dim {
    /// The power of length.
    pub length: i8,
    /// The power of angle.
    pub angle: i8,
}

impl Dim {
    /// A plain number.
    pub const NONE: Dim = Dim {
        length: 0,
        angle: 0,
    };
    /// A length, µm.
    pub const LENGTH: Dim = Dim {
        length: 1,
        angle: 0,
    };
    /// An angle, radians.
    pub const ANGLE: Dim = Dim {
        length: 0,
        angle: 1,
    };

    /// Whether a value of this dimension can stand where `other` is expected: the same power of
    /// length, and of angle unless either is a plain number (radians are numbers).
    pub fn fits(self, other: Dim) -> bool {
        self.length == other.length
            && (self.angle == other.angle || self.angle == 0 || other.angle == 0)
    }

    /// Its name in a sentence: "a length", "an angle", "a number", "an area", "length^3".
    pub fn name(self) -> String {
        match (self.length, self.angle) {
            (0, 0) => "a number".into(),
            (1, 0) => "a length".into(),
            (2, 0) => "an area".into(),
            (0, 1) => "an angle".into(),
            (-1, 0) => "an inverse length".into(),
            (l, 0) => format!("length^{l}"),
            (0, a) => format!("angle^{a}"),
            (l, a) => format!("length^{l} angle^{a}"),
        }
    }
}

impl fmt::Display for Dim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name())
    }
}

/// An expression's error: what is wrong and the column (from 1, in characters) where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExprError {
    /// The column, from 1.
    pub column: usize,
    /// What is wrong.
    pub message: String,
}

impl ExprError {
    pub(crate) fn at(column: usize, message: impl Into<String>) -> ExprError {
        ExprError {
            column,
            message: message.into(),
        }
    }
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "column {}: {}", self.column, self.message)
    }
}

impl std::error::Error for ExprError {}

/// A value and its derivative along one direction of the parameters.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dual {
    /// The value.
    pub value: f64,
    /// Its derivative.
    pub derivative: f64,
}

impl Dual {
    /// A constant: derivative zero.
    pub const fn constant(value: f64) -> Dual {
        Dual {
            value,
            derivative: 0.0,
        }
    }

    /// The variable itself: derivative one.
    pub const fn variable(value: f64) -> Dual {
        Dual {
            value,
            derivative: 1.0,
        }
    }
}

/// The names an expression can use, each with a slot in the values it is evaluated at: the
/// coordinates `x`, `y`, `z`, `t`, `s`, `i`, `j` in slots 0 to 6, then the parameters in the
/// order they were defined.
#[derive(Clone, Debug, PartialEq)]
pub struct Scope {
    names: Vec<(String, Option<Dim>)>,
}

/// The coordinates, in their slots, with their dimensions.
pub const COORDINATES: [(&str, Dim); 7] = [
    ("x", Dim::LENGTH),
    ("y", Dim::LENGTH),
    ("z", Dim::LENGTH),
    ("t", Dim::NONE),
    ("s", Dim::LENGTH),
    ("i", Dim::NONE),
    ("j", Dim::NONE),
];

/// Names a parameter can't take: the language's own.
pub(crate) fn reserved(name: &str) -> bool {
    parse::FUNCTIONS.iter().any(|f| f.0 == name)
        || parse::UNITS.iter().any(|u| u.0 == name)
        || COORDINATES.iter().any(|c| c.0 == name)
        || matches!(name, "pi" | "if" | "piecewise")
}

impl Default for Scope {
    fn default() -> Scope {
        Scope::new()
    }
}

impl Scope {
    /// The coordinates only.
    pub fn new() -> Scope {
        Scope {
            names: COORDINATES
                .iter()
                .map(|(n, d)| ((*n).to_owned(), Some(*d)))
                .collect(),
        }
    }

    /// Adds the parameter `name` of dimension `dim` (`None`: a bare zero, which fits any) and
    /// returns its slot.
    ///
    /// # Errors
    ///
    /// The reason, unless the name is a letter or `_` followed by letters, digits and `_`, not
    /// the language's own and not defined already.
    pub fn define(&mut self, name: &str, dim: Option<Dim>) -> Result<usize, String> {
        let mut chars = name.chars();
        let ok = chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !ok {
            return Err(format!(
                "\"{name}\" isn't a name: a letter or _ followed by letters, digits and _"
            ));
        }
        if reserved(name) {
            return Err(format!(
                "\"{name}\" is the expression language's own (a function, unit, constant or \
                 coordinate)"
            ));
        }
        if self.slot(name).is_some() {
            return Err(format!("\"{name}\" is defined twice"));
        }
        self.names.push((name.to_owned(), dim));
        Ok(self.names.len() - 1)
    }

    /// Sets the dimension of the name in `slot`.
    pub(crate) fn set_dim(&mut self, slot: usize, dim: Option<Dim>) {
        self.names[slot].1 = dim;
    }

    /// The slot of `name`.
    pub fn slot(&self, name: &str) -> Option<usize> {
        self.names.iter().position(|(n, _)| n == name)
    }

    /// The dimension of the name in `slot` (`None`: any).
    pub fn dim(&self, slot: usize) -> Option<Dim> {
        self.names[slot].1
    }

    /// How many slots: the values an expression is evaluated at need this many.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether it has no names (never: the coordinates are always there).
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The name in `slot`.
    pub fn name(&self, slot: usize) -> &str {
        &self.names[slot].0
    }
}

/// An error in a parameter table: which parameter, and what is wrong with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterError {
    /// The parameter, or empty for the table as a whole (a cycle).
    pub name: String,
    /// What is wrong, with its column when it is in the definition.
    pub message: String,
}

impl fmt::Display for ParameterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.name.is_empty() {
            f.write_str(&self.message)
        } else {
            write!(f, "{}: {}", self.name, self.message)
        }
    }
}

impl std::error::Error for ParameterError {}

/// Named parameters, each an expression of the others: `w = "500 nm"`, `gap = "w/2.5"`.
/// They are evaluated in an order where each comes after those it uses (ties by the order
/// given); a cycle is refused.
#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    scope: Scope,
    /// each parameter's slot, definition and expression, in the order given
    defined: Vec<(usize, String, Expr)>,
    /// indices into `defined`, in the order they are evaluated
    order: Vec<usize>,
    /// parameters set to a value, with it
    fixed: Vec<(usize, f64)>,
    /// every slot's value: the coordinates 0, then the parameters
    values: Vec<f64>,
}

impl Parameters {
    /// No parameters.
    pub fn none() -> Parameters {
        Parameters {
            scope: Scope::new(),
            defined: Vec::new(),
            order: Vec::new(),
            fixed: Vec::new(),
            values: vec![0.0; COORDINATES.len()],
        }
    }

    /// The parameters `definitions`, (name, expression) pairs.
    ///
    /// # Errors
    ///
    /// A [`ParameterError`] for a bad or repeated name, an expression that doesn't parse or
    /// whose units don't add up, a cycle, or a value that isn't finite (a division by zero, a
    /// square root of a negative number).
    pub fn new(definitions: &[(String, String)]) -> Result<Parameters, ParameterError> {
        let mut scope = Scope::new();
        let err = |name: &str, message: String| ParameterError {
            name: name.to_owned(),
            message,
        };
        let mut slots = Vec::new();
        for (name, _) in definitions {
            slots.push(scope.define(name, None).map_err(|m| err(name, m))?);
        }
        // who uses whom
        let uses: Vec<Vec<usize>> = definitions
            .iter()
            .map(|(_, text)| {
                parse::identifiers(text)
                    .into_iter()
                    .filter_map(|n| definitions.iter().position(|(m, _)| *m == n))
                    .collect()
            })
            .collect();
        let n = definitions.len();
        let mut order = Vec::with_capacity(n);
        let mut done = vec![false; n];
        while order.len() < n {
            let next = (0..n).find(|&k| !done[k] && uses[k].iter().all(|&u| done[u] || u == k));
            match next {
                Some(k) if !uses[k].contains(&k) => {
                    done[k] = true;
                    order.push(k);
                }
                Some(k) => {
                    return Err(err(
                        &definitions[k].0,
                        format!("uses itself: {} = {}", definitions[k].0, definitions[k].1),
                    ));
                }
                None => {
                    let left: Vec<usize> = (0..n).filter(|&k| !done[k]).collect();
                    return Err(ParameterError {
                        name: String::new(),
                        message: format!(
                            "the parameters {} use each other in a cycle: {}",
                            list(left.iter().map(|&k| definitions[k].0.as_str())),
                            cycle(&left, &uses, definitions)
                        ),
                    });
                }
            }
        }
        let mut compiled: Vec<Option<Expr>> = vec![None; n];
        for &k in &order {
            let (name, text) = &definitions[k];
            let e = Expr::parse(text, &scope).map_err(|e| err(name, e.to_string()))?;
            scope.set_dim(slots[k], e.dim());
            compiled[k] = Some(e);
        }
        let defined = definitions
            .iter()
            .zip(slots)
            .zip(compiled)
            .filter_map(|(((_, text), slot), e)| Some((slot, text.clone(), e?)))
            .collect();
        let mut p = Parameters {
            values: vec![0.0; scope.len()],
            scope,
            defined,
            order,
            fixed: Vec::new(),
        };
        p.evaluate()?;
        Ok(p)
    }

    fn evaluate(&mut self) -> Result<(), ParameterError> {
        for &k in &self.order {
            let (slot, _, e) = &self.defined[k];
            let value = match self.fixed.iter().find(|f| f.0 == *slot) {
                Some(&(_, v)) => v,
                None => e.try_eval(&self.values).map_err(|e| ParameterError {
                    name: self.scope.name(*slot).to_owned(),
                    message: e.to_string(),
                })?,
            };
            self.values[*slot] = value;
        }
        Ok(())
    }

    /// The parameters with `name` set to `value` (in µm for a length, radians for an angle)
    /// instead of its definition, and those that use it evaluated again: a sweep's point.
    ///
    /// # Errors
    ///
    /// A [`ParameterError`] for an unknown name or a value that isn't finite, or one the
    /// others can't be evaluated at.
    pub fn with(&self, name: &str, value: f64) -> Result<Parameters, ParameterError> {
        let err = |message: String| ParameterError {
            name: name.to_owned(),
            message,
        };
        let slot = self
            .slot(name)
            .ok_or_else(|| err("isn't a parameter".to_owned()))?;
        if !value.is_finite() {
            return Err(err(format!("can't be set to {value}")));
        }
        let mut p = self.clone();
        p.fixed.retain(|f| f.0 != slot);
        p.fixed.push((slot, value));
        p.evaluate()?;
        Ok(p)
    }

    /// The names an expression can use: the coordinates and these parameters.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    /// The values to evaluate an expression of [`Parameters::scope`] at: the coordinates 0,
    /// then the parameters'.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// The parameters' names, in the order given.
    pub fn names(&self) -> Vec<&str> {
        self.defined
            .iter()
            .map(|(slot, _, _)| self.scope.name(*slot))
            .collect()
    }

    /// The slot of the parameter `name`, if it is one.
    pub fn slot(&self, name: &str) -> Option<usize> {
        self.scope.slot(name).filter(|&s| s >= COORDINATES.len())
    }

    /// The value of the parameter `name`.
    pub fn value(&self, name: &str) -> Option<f64> {
        self.slot(name).map(|s| self.values[s])
    }

    /// The dimension of the parameter `name` (`None` inside: a bare zero, which fits any).
    pub fn dim(&self, name: &str) -> Option<Option<Dim>> {
        self.slot(name).map(|s| self.scope.dim(s))
    }

    /// Every slot's value with its derivative with respect to the parameter `name`, by forward
    /// mode: `name` itself 1, those that use it by the chain rule, the others 0.
    ///
    /// # Errors
    ///
    /// A [`ParameterError`] if `name` isn't a parameter.
    pub fn derivatives(&self, name: &str) -> Result<Vec<Dual>, ParameterError> {
        let seed = self.slot(name).ok_or_else(|| ParameterError {
            name: name.to_owned(),
            message: "isn't a parameter".to_owned(),
        })?;
        let mut duals: Vec<Dual> = self.values.iter().map(|v| Dual::constant(*v)).collect();
        for &k in &self.order {
            let (slot, _, e) = &self.defined[k];
            duals[*slot] = if *slot == seed {
                Dual::variable(self.values[*slot])
            } else if let Some(&(_, v)) = self.fixed.iter().find(|f| f.0 == *slot) {
                Dual::constant(v)
            } else {
                e.eval_dual(&duals)
            };
        }
        Ok(duals)
    }
}

/// "a, b and c".
fn list<'a>(names: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<&str> = names.collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// One cycle among the parameters `left` (each of which uses another of them), as
/// "a -> b -> a".
fn cycle(left: &[usize], uses: &[Vec<usize>], definitions: &[(String, String)]) -> String {
    let mut path = vec![left[0]];
    loop {
        let here = *path.last().expect("a start");
        let Some(&next) = uses[here].iter().find(|u| left.contains(u)) else {
            break;
        };
        if let Some(at) = path.iter().position(|&p| p == next) {
            let mut names: Vec<&str> = path[at..]
                .iter()
                .map(|&k| definitions[k].0.as_str())
                .collect();
            names.push(definitions[next].0.as_str());
            return names.join(" -> ");
        }
        path.push(next);
    }
    list(left.iter().map(|&k| definitions[k].0.as_str()))
}

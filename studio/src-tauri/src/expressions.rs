//! What an expression may say, for the job editor's popover (#430): the expression language's
//! functions, units, constants and coordinates (photonoxide::expr, docs/methods/expressions.md),
//! each with an example the library parses, and the parameters in scope with their values, as the
//! library evaluates them.

use photonoxide::expr::Parameters;
use serde::Serialize;

/// A name an expression may use.
#[derive(Debug, Serialize)]
pub struct Word {
    pub name: &'static str,
    /// "function", "unit", "constant" or "coordinate".
    pub kind: &'static str,
    /// How it is written: `atan2(y, x)`, `500 nm`.
    pub usage: &'static str,
    pub doc: &'static str,
}

const fn w(name: &'static str, kind: &'static str, usage: &'static str, doc: &'static str) -> Word {
    Word {
        name,
        kind,
        usage,
        doc,
    }
}

/// The expression language's words, as its module's docs list them; each `usage` parses.
#[rustfmt::skip]
pub const WORDS: &[Word] = &[
    w("sin", "function", "sin(a)", "The sine; of an angle or a number (radians)."),
    w("cos", "function", "cos(a)", "The cosine; of an angle or a number (radians)."),
    w("tan", "function", "tan(a)", "The tangent; of an angle or a number (radians)."),
    w("asin", "function", "asin(v)", "The arcsine, an angle."),
    w("acos", "function", "acos(v)", "The arccosine, an angle."),
    w("atan", "function", "atan(v)", "The arctangent, an angle."),
    w("atan2", "function", "atan2(v, u)", "The angle of the point (u, v), in (−π, π]."),
    w("exp", "function", "exp(v)", "e to the v."),
    w("ln", "function", "ln(v)", "The natural logarithm."),
    w("log10", "function", "log10(v)", "The base-10 logarithm."),
    w("sqrt", "function", "sqrt(v)", "The square root; halves the powers of its units."),
    w("abs", "function", "abs(v)", "The absolute value."),
    w("min", "function", "min(a, b)", "The smaller of two values of one dimension."),
    w("max", "function", "max(a, b)", "The larger of two values of one dimension."),
    w("clamp", "function", "clamp(v, lo, hi)", "v held between lo and hi."),
    w("floor", "function", "floor(v)", "The largest integer not above v."),
    w("ceil", "function", "ceil(v)", "The smallest integer not below v."),
    w("round", "function", "round(v)", "The nearest integer, halves away from zero."),
    w("erf", "function", "erf(v)", "The error function."),
    w("tanh", "function", "tanh(v)", "The hyperbolic tangent."),
    w("sinc", "function", "sinc(v)", "sin(v)/v, 1 at 0."),
    w("smoothstep", "function", "smoothstep(e0, e1, v)", "3u² − 2u³ for u = (v − e0)/(e1 − e0) clamped to [0, 1]."),
    w("if", "function", "if(c, a, b)", "a where c isn't 0, b where it is."),
    w("piecewise", "function", "piecewise(c1, a1, c2, a2, otherwise)", "The first a whose c isn't 0, or the last value."),
    w("spline", "function", "spline(v, [0, 1, 2, 3], [0, 1, 4, 9])", "The not-a-knot cubic spline through the points: the knots constants, increasing; the values any expressions."),
    w("nm", "unit", "500 nm", "Nanometres, a length (kept in µm)."),
    w("um", "unit", "0.5 um", "Micrometres, a length; also written µm."),
    w("µm", "unit", "0.5 µm", "Micrometres, a length."),
    w("mm", "unit", "1.2 mm", "Millimetres, a length (kept in µm)."),
    w("deg", "unit", "90 deg", "Degrees, an angle (kept in radians)."),
    w("rad", "unit", "1.5 rad", "Radians, an angle."),
    w("pi", "constant", "2*pi", "π."),
    w("x", "coordinate", "x", "The x coordinate, a length, where a definition supplies it."),
    w("y", "coordinate", "y", "The y coordinate, a length, where a definition supplies it."),
    w("z", "coordinate", "z", "The z coordinate, a length, where a definition supplies it."),
    w("s", "coordinate", "s", "The arc length along a path, a length."),
    w("t", "coordinate", "t", "A curve's parameter, a number from 0 to 1."),
    w("i", "coordinate", "i", "An array's first index, a number from 0."),
    w("j", "coordinate", "j", "An array's second index, a number from 0."),
];

/// A parameter in scope, with its value as the library evaluates it.
#[derive(Debug, Serialize)]
pub struct ScopeParameter {
    pub name: String,
    /// Its definition, as the text has it.
    pub text: String,
    /// In µm for a length, radians for an angle; none when the table doesn't evaluate.
    pub value: Option<f64>,
    /// "a length", "an angle", "a number", …
    pub dimension: String,
}

/// What an expression may use at the cursor.
#[derive(Debug, Serialize)]
pub struct ExpressionScope {
    pub words: &'static [Word],
    pub parameters: Vec<ScopeParameter>,
    /// Why the parameters don't evaluate, when they don't.
    pub error: Option<String>,
}

/// The expression language's words, and the parameters `definitions` (name, expression), each
/// with its value.
#[tauri::command]
pub fn expression_scope(definitions: Vec<(String, String)>) -> ExpressionScope {
    let (values, error) = match Parameters::new(&definitions) {
        Ok(p) => (Some(p), None),
        Err(e) => (None, Some(e.to_string())),
    };
    let parameters = definitions
        .into_iter()
        .map(|(name, text)| {
            let value = values.as_ref().and_then(|p| p.value(&name));
            let dimension = values
                .as_ref()
                .and_then(|p| p.dim(&name).flatten())
                .map(|d| d.name())
                .unwrap_or_default();
            ScopeParameter {
                name,
                text,
                value,
                dimension,
            }
        })
        .collect();
    ExpressionScope {
        words: WORDS,
        parameters,
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use photonoxide::expr::{Dim, Expr, Scope};

    #[test]
    fn every_word_is_the_librarys() {
        // the usages' placeholders, plain numbers
        let mut scope = Scope::new();
        for name in [
            "a",
            "b",
            "c",
            "u",
            "v",
            "lo",
            "hi",
            "e0",
            "e1",
            "c1",
            "a1",
            "c2",
            "a2",
            "otherwise",
        ] {
            scope.define(name, Some(Dim::NONE)).unwrap();
        }
        for word in WORDS {
            let e = Expr::parse(word.usage, &scope);
            assert!(e.is_ok(), "{}: {e:?}", word.usage);
            assert!(word.usage.contains(word.name), "{}", word.name);
        }
        // a name the library doesn't know is refused, so the list is the language's
        assert!(Expr::parse("sinh(1)", &scope).is_err());
        assert!(Expr::parse("2 furlongs", &scope).is_err());
    }

    #[test]
    fn parameters_come_with_their_values() {
        let s = expression_scope(vec![
            ("w".into(), "500 nm".into()),
            ("gap".into(), "w/2".into()),
        ]);
        assert!(s.error.is_none());
        assert_eq!(s.parameters[0].value, Some(0.5));
        assert_eq!(s.parameters[1].value, Some(0.25));
        assert_eq!(s.parameters[1].dimension, "a length");
        let bad = expression_scope(vec![("w".into(), "w + 1".into())]);
        assert!(bad.error.is_some() && bad.parameters[0].value.is_none());
    }
}

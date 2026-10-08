//! The expression language's parser, its checks of units, and the compiled program.

use std::f64::consts::PI;

use super::special::{erf, erf_derivative};
use super::{Dim, Dual, ExprError, Scope};

/// The functions, with how many arguments they take.
pub(crate) const FUNCTIONS: [(&str, usize); 22] = [
    ("sin", 1),
    ("cos", 1),
    ("tan", 1),
    ("asin", 1),
    ("acos", 1),
    ("atan", 1),
    ("atan2", 2),
    ("exp", 1),
    ("ln", 1),
    ("log10", 1),
    ("sqrt", 1),
    ("abs", 1),
    ("min", 2),
    ("max", 2),
    ("clamp", 3),
    ("floor", 1),
    ("ceil", 1),
    ("round", 1),
    ("erf", 1),
    ("tanh", 1),
    ("sinc", 1),
    ("smoothstep", 3),
];

/// The units: their names, dimensions and how a number in them becomes one in µm or radians.
pub(crate) const UNITS: [(&str, Dim, Scale); 6] = [
    ("nm", Dim::LENGTH, Scale::Div(1000.0)),
    ("um", Dim::LENGTH, Scale::Mul(1.0)),
    ("µm", Dim::LENGTH, Scale::Mul(1.0)),
    ("mm", Dim::LENGTH, Scale::Mul(1000.0)),
    ("deg", Dim::ANGLE, Scale::Degrees),
    ("rad", Dim::ANGLE, Scale::Mul(1.0)),
];

/// How a unit converts: a division keeps nanometres exact as `Length::nm` does them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Scale {
    Mul(f64),
    Div(f64),
    Degrees,
}

impl Scale {
    fn apply(self, v: f64) -> f64 {
        match self {
            Scale::Mul(m) => v * m,
            Scale::Div(d) => v / d,
            Scale::Degrees => v.to_radians(),
        }
    }
}

/// The deepest stack a program may need.
const DEPTH: usize = 64;

/// The points [`Expr::eval_many`] evaluates at once.
const BLOCK: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Name,
    Op(char),
    Le,
    Ge,
    EqEq,
    Ne,
    End,
}

#[derive(Clone, Debug)]
struct Token {
    tok: Tok,
    text: String,
    col: usize,
}

fn lex(text: &str) -> Result<Vec<Token>, ExprError> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut k = 0;
    while k < chars.len() {
        let c = chars[k];
        let col = k + 1;
        if c.is_whitespace() {
            k += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(k + 1).is_some_and(char::is_ascii_digit)) {
            let start = k;
            while k < chars.len() && (chars[k].is_ascii_digit() || chars[k] == '.') {
                k += 1;
            }
            // an exponent: e or E, a sign, digits (not the e of a name after the number)
            if k < chars.len() && (chars[k] == 'e' || chars[k] == 'E') {
                let mut m = k + 1;
                if m < chars.len() && (chars[m] == '+' || chars[m] == '-') {
                    m += 1;
                }
                if m < chars.len() && chars[m].is_ascii_digit() {
                    while m < chars.len() && chars[m].is_ascii_digit() {
                        m += 1;
                    }
                    k = m;
                }
            }
            let s: String = chars[start..k].iter().collect();
            let v: f64 = s
                .parse()
                .map_err(|_| ExprError::at(col, format!("\"{s}\" isn't a number")))?;
            out.push(Token {
                tok: Tok::Num(v),
                text: s,
                col,
            });
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = k;
            while k < chars.len() && (chars[k].is_alphanumeric() || chars[k] == '_') {
                k += 1;
            }
            out.push(Token {
                tok: Tok::Name,
                text: chars[start..k].iter().collect(),
                col,
            });
            continue;
        }
        let two = |a: char, b: char| c == a && chars.get(k + 1) == Some(&b);
        let (tok, len) = if two('<', '=') {
            (Tok::Le, 2)
        } else if two('>', '=') {
            (Tok::Ge, 2)
        } else if two('=', '=') {
            (Tok::EqEq, 2)
        } else if two('!', '=') {
            (Tok::Ne, 2)
        } else if two('*', '*') {
            (Tok::Op('^'), 2)
        } else if "+-*/^()<>,".contains(c) {
            (Tok::Op(c), 1)
        } else if c == '×' || c == '·' {
            (Tok::Op('*'), 1)
        } else if c == '−' {
            (Tok::Op('-'), 1)
        } else {
            return Err(ExprError::at(col, format!("unexpected \"{c}\"")));
        };
        out.push(Token {
            tok,
            text: chars[k..k + len].iter().collect(),
            col,
        });
        k += len;
    }
    out.push(Token {
        tok: Tok::End,
        text: String::new(),
        col: chars.len() + 1,
    });
    Ok(out)
}

/// The names an expression's text mentions, in order (for ordering parameters before parsing
/// them); empty if it doesn't lex.
pub(crate) fn identifiers(text: &str) -> Vec<String> {
    lex(text)
        .map(|toks| {
            toks.into_iter()
                .filter(|t| t.tok == Tok::Name)
                .map(|t| t.text)
                .collect()
        })
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Bin {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum F1 {
    Neg,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Exp,
    Ln,
    Log10,
    Sqrt,
    Abs,
    Floor,
    Ceil,
    Round,
    Erf,
    Tanh,
    Sinc,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum F2 {
    Atan2,
    Min,
    Max,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum F3 {
    Clamp,
    Smoothstep,
}

/// A parsed expression: each node with its column, and its dimension once checked (`None`: a
/// bare zero, which fits any).
#[derive(Clone, Debug)]
enum Node {
    Num(f64),
    Load(usize),
    Unary(F1, Box<Typed>),
    Binary(Bin, Box<Typed>, Box<Typed>),
    Call2(F2, Box<Typed>, Box<Typed>),
    Call3(F3, Box<Typed>, Box<Typed>, Box<Typed>),
    If(Box<Typed>, Box<Typed>, Box<Typed>),
}

#[derive(Clone, Debug)]
struct Typed {
    node: Node,
    dim: Option<Dim>,
    col: usize,
}

struct Parser<'a> {
    toks: Vec<Token>,
    k: usize,
    scope: &'a Scope,
}

fn fits(a: Option<Dim>, b: Option<Dim>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.fits(b),
        _ => true,
    }
}

/// The dimension of a sum of `a` and `b` (which fit): an angle if either is one.
fn join(a: Option<Dim>, b: Option<Dim>) -> Option<Dim> {
    match (a, b) {
        (Some(a), Some(b)) => Some(Dim {
            length: a.length,
            angle: if a.angle != 0 { a.angle } else { b.angle },
        }),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    }
}

fn name(d: Option<Dim>) -> String {
    d.map_or("zero".to_owned(), Dim::name)
}

impl Parser<'_> {
    fn peek(&self) -> &Token {
        &self.toks[self.k]
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.k].clone();
        if self.k + 1 < self.toks.len() {
            self.k += 1;
        }
        t
    }

    fn expect(&mut self, c: char, what: &str) -> Result<(), ExprError> {
        let t = self.peek();
        if t.tok == Tok::Op(c) {
            self.bump();
            Ok(())
        } else {
            Err(ExprError::at(
                t.col,
                format!("expected {what}, found {}", shown(t)),
            ))
        }
    }

    fn expr(&mut self) -> Result<Typed, ExprError> {
        let a = self.sum()?;
        let op = match self.peek().tok {
            Tok::Op('<') => Bin::Lt,
            Tok::Le => Bin::Le,
            Tok::Op('>') => Bin::Gt,
            Tok::Ge => Bin::Ge,
            Tok::EqEq => Bin::Eq,
            Tok::Ne => Bin::Ne,
            _ => return Ok(a),
        };
        let t = self.bump();
        let b = self.sum()?;
        if !fits(a.dim, b.dim) {
            return Err(ExprError::at(
                t.col,
                format!("can't compare {} with {}", name(a.dim), name(b.dim)),
            ));
        }
        Ok(binary(op, a, b, Some(Dim::NONE), t.col))
    }

    fn sum(&mut self) -> Result<Typed, ExprError> {
        let mut a = self.product()?;
        loop {
            let op = match self.peek().tok {
                Tok::Op('+') => Bin::Add,
                Tok::Op('-') => Bin::Sub,
                _ => return Ok(a),
            };
            let t = self.bump();
            let b = self.product()?;
            if !fits(a.dim, b.dim) {
                let verb = if op == Bin::Add { "add" } else { "subtract" };
                let (to, from) = if op == Bin::Add {
                    ("to", &b)
                } else {
                    ("from", &b)
                };
                return Err(ExprError::at(
                    t.col,
                    format!("can't {verb} {} {to} {}", name(from.dim), name(a.dim)),
                ));
            }
            let dim = join(a.dim, b.dim);
            a = binary(op, a, b, dim, t.col);
        }
    }

    fn product(&mut self) -> Result<Typed, ExprError> {
        let mut a = self.unary()?;
        loop {
            let op = match self.peek().tok {
                Tok::Op('*') => Bin::Mul,
                Tok::Op('/') => Bin::Div,
                _ => return Ok(a),
            };
            let t = self.bump();
            let b = self.unary()?;
            let dim = match (a.dim, b.dim, op) {
                (_, None, Bin::Div) => {
                    return Err(ExprError::at(t.col, "division by zero"));
                }
                (None, _, _) | (_, None, _) => None,
                (Some(x), Some(y), Bin::Mul) => Some(Dim {
                    length: x.length + y.length,
                    angle: x.angle + y.angle,
                }),
                (Some(x), Some(y), _) => Some(Dim {
                    length: x.length - y.length,
                    angle: x.angle - y.angle,
                }),
            };
            if let (Bin::Div, Node::Num(v)) = (op, &b.node)
                && *v == 0.0
            {
                return Err(ExprError::at(t.col, "division by zero"));
            }
            a = binary(op, a, b, dim, t.col);
        }
    }

    fn unary(&mut self) -> Result<Typed, ExprError> {
        match self.peek().tok {
            Tok::Op('-') => {
                let t = self.bump();
                let a = self.unary()?;
                let dim = a.dim;
                Ok(fold(Typed {
                    node: Node::Unary(F1::Neg, Box::new(a)),
                    dim,
                    col: t.col,
                }))
            }
            Tok::Op('+') => {
                self.bump();
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<Typed, ExprError> {
        let base = self.primary()?;
        if self.peek().tok != Tok::Op('^') {
            return Ok(base);
        }
        let t = self.bump();
        let exp = self.unary()?;
        if !fits(exp.dim, Some(Dim::NONE)) || exp.dim.is_some_and(|d| d.angle != 0) {
            return Err(ExprError::at(
                exp.col,
                format!("an exponent must be a number, not {}", name(exp.dim)),
            ));
        }
        let dim = match base.dim {
            Some(d) if d != Dim::NONE => {
                let Node::Num(k) = exp.node else {
                    return Err(ExprError::at(
                        exp.col,
                        format!(
                            "{} can be raised only to a constant power, its dimension depends on it",
                            d.name()
                        ),
                    ));
                };
                let (l, a) = (f64::from(d.length) * k, f64::from(d.angle) * k);
                if l.fract() != 0.0 || a.fract() != 0.0 || l.abs() > 100.0 || a.abs() > 100.0 {
                    return Err(ExprError::at(
                        exp.col,
                        format!(
                            "{} to the power {k} isn't a whole power of a unit",
                            d.name()
                        ),
                    ));
                }
                Some(Dim {
                    length: l as i8,
                    angle: a as i8,
                })
            }
            other => other,
        };
        Ok(binary(Bin::Pow, base, exp, dim, t.col))
    }

    fn args(&mut self, name: &str) -> Result<Vec<Typed>, ExprError> {
        self.expect('(', &format!("\"(\" after {name}"))?;
        let mut args = vec![self.expr()?];
        while self.peek().tok == Tok::Op(',') {
            self.bump();
            args.push(self.expr()?);
        }
        self.expect(')', "\",\" or \")\"")?;
        Ok(args)
    }

    fn primary(&mut self) -> Result<Typed, ExprError> {
        let t = self.bump();
        match t.tok {
            Tok::Num(v) => {
                // a unit right after the number
                if self.peek().tok == Tok::Name
                    && let Some(u) = UNITS.iter().find(|u| u.0 == self.peek().text)
                {
                    self.bump();
                    // a power of the unit alone: 4 um^2 is 4 µm², not (4 µm)²
                    if self.peek().tok == Tok::Op('^') {
                        let at = self.bump().col;
                        let k = self.unary()?;
                        let Node::Num(k) = k.node else {
                            return Err(ExprError::at(at, "a unit's power must be a constant"));
                        };
                        if k.fract() != 0.0 || k.abs() > 100.0 {
                            return Err(ExprError::at(
                                at,
                                format!("a unit's power must be a whole number, not {k}"),
                            ));
                        }
                        let k = k as i32;
                        return Ok(Typed {
                            node: Node::Num(v * u.2.apply(1.0).powi(k)),
                            dim: Some(Dim {
                                length: u.1.length * k as i8,
                                angle: u.1.angle * k as i8,
                            }),
                            col: t.col,
                        });
                    }
                    return Ok(Typed {
                        node: Node::Num(u.2.apply(v)),
                        dim: Some(u.1),
                        col: t.col,
                    });
                }
                Ok(Typed {
                    node: Node::Num(v),
                    dim: if v == 0.0 { None } else { Some(Dim::NONE) },
                    col: t.col,
                })
            }
            Tok::Op('(') => {
                let e = self.expr()?;
                self.expect(')', "\")\"")?;
                Ok(Typed { col: t.col, ..e })
            }
            Tok::Name => self.named(&t),
            _ => Err(ExprError::at(
                t.col,
                format!("expected a number, a name or \"(\", found {}", shown(&t)),
            )),
        }
    }

    fn named(&mut self, t: &Token) -> Result<Typed, ExprError> {
        let col = t.col;
        let n = t.text.as_str();
        if self.peek().tok == Tok::Op('(') {
            return self.call(n, col);
        }
        if n == "pi" {
            return Ok(Typed {
                node: Node::Num(PI),
                dim: Some(Dim::NONE),
                col,
            });
        }
        if let Some(u) = UNITS.iter().find(|u| u.0 == n) {
            return Ok(Typed {
                node: Node::Num(u.2.apply(1.0)),
                dim: Some(u.1),
                col,
            });
        }
        if let Some(slot) = self.scope.slot(n) {
            return Ok(Typed {
                node: Node::Load(slot),
                dim: self.scope.dim(slot),
                col,
            });
        }
        if FUNCTIONS.iter().any(|f| f.0 == n) || n == "if" || n == "piecewise" {
            return Err(ExprError::at(col, format!("{n} is a function: {n}(...)")));
        }
        Err(ExprError::at(col, format!("unknown name \"{n}\"")))
    }

    fn call(&mut self, n: &str, col: usize) -> Result<Typed, ExprError> {
        let args = self.args(n)?;
        let count = |want: usize| -> Result<(), ExprError> {
            if args.len() == want {
                Ok(())
            } else {
                Err(ExprError::at(
                    col,
                    format!(
                        "{n} takes {want} argument{}, got {}",
                        if want == 1 { "" } else { "s" },
                        args.len()
                    ),
                ))
            }
        };
        if n == "if" {
            count(3)?;
            let mut it = args.into_iter();
            let (c, a, b) = (it.next(), it.next(), it.next());
            let (Some(c), Some(a), Some(b)) = (c, a, b) else {
                unreachable!("three arguments")
            };
            return branch(c, a, b, col);
        }
        if n == "piecewise" {
            if args.len() < 3 || args.len() % 2 == 0 {
                return Err(ExprError::at(
                    col,
                    format!(
                        "piecewise takes conditions and values in pairs and a last value, an odd \
                         number of at least 3 arguments, got {}",
                        args.len()
                    ),
                ));
            }
            let mut it = args.into_iter().rev();
            let mut acc = it.next().expect("a last value");
            let rest: Vec<Typed> = it.collect();
            for pair in rest.chunks(2) {
                // (reversed: the value, then its condition)
                acc = branch(pair[1].clone(), pair[0].clone(), acc, col)?;
            }
            return Ok(acc);
        }
        let Some(&(_, want)) = FUNCTIONS.iter().find(|f| f.0 == n) else {
            return Err(ExprError::at(col, format!("unknown function \"{n}\"")));
        };
        count(want)?;
        let mut it = args.into_iter();
        let a = it.next().expect("an argument");
        let need = |arg: &Typed, ok: bool, what: &str| -> Result<(), ExprError> {
            if ok {
                Ok(())
            } else {
                Err(ExprError::at(
                    arg.col,
                    format!("{n} takes {what}, not {}", name(arg.dim)),
                ))
            }
        };
        let number = |d: Option<Dim>| d.is_none_or(|d| d.length == 0 && d.angle == 0);
        let unary = |f: F1, a: Typed, dim: Option<Dim>| {
            Ok(fold(Typed {
                node: Node::Unary(f, Box::new(a)),
                dim,
                col,
            }))
        };
        match want {
            1 => {
                let f = match n {
                    "sin" => F1::Sin,
                    "cos" => F1::Cos,
                    "tan" => F1::Tan,
                    "asin" => F1::Asin,
                    "acos" => F1::Acos,
                    "atan" => F1::Atan,
                    "exp" => F1::Exp,
                    "ln" => F1::Ln,
                    "log10" => F1::Log10,
                    "sqrt" => F1::Sqrt,
                    "abs" => F1::Abs,
                    "floor" => F1::Floor,
                    "ceil" => F1::Ceil,
                    "round" => F1::Round,
                    "erf" => F1::Erf,
                    "tanh" => F1::Tanh,
                    _ => F1::Sinc,
                };
                match f {
                    F1::Sin | F1::Cos | F1::Tan | F1::Sinc => {
                        let ok = a.dim.is_none_or(|d| d.length == 0 && d.angle <= 1);
                        need(&a, ok, "an angle or a number")?;
                        unary(f, a, Some(Dim::NONE))
                    }
                    F1::Asin | F1::Acos | F1::Atan => {
                        need(&a, number(a.dim), "a number")?;
                        unary(f, a, Some(Dim::ANGLE))
                    }
                    F1::Sqrt => {
                        let dim = match a.dim {
                            Some(d) => {
                                if d.length % 2 != 0 || d.angle % 2 != 0 {
                                    return Err(ExprError::at(
                                        a.col,
                                        format!("sqrt of {} has no unit", d.name()),
                                    ));
                                }
                                Some(Dim {
                                    length: d.length / 2,
                                    angle: d.angle / 2,
                                })
                            }
                            None => None,
                        };
                        unary(f, a, dim)
                    }
                    F1::Abs | F1::Floor | F1::Ceil | F1::Round => {
                        let dim = a.dim;
                        unary(f, a, dim)
                    }
                    _ => {
                        need(&a, number(a.dim), "a number")?;
                        unary(f, a, Some(Dim::NONE))
                    }
                }
            }
            2 => {
                let b = it.next().expect("two arguments");
                if !fits(a.dim, b.dim) {
                    return Err(ExprError::at(
                        b.col,
                        format!(
                            "{n} takes two of a kind, not {} and {}",
                            name(a.dim),
                            name(b.dim)
                        ),
                    ));
                }
                let (f, dim) = match n {
                    "atan2" => (F2::Atan2, Some(Dim::ANGLE)),
                    "min" => (F2::Min, join(a.dim, b.dim)),
                    _ => (F2::Max, join(a.dim, b.dim)),
                };
                Ok(fold(Typed {
                    node: Node::Call2(f, Box::new(a), Box::new(b)),
                    dim,
                    col,
                }))
            }
            _ => {
                let b = it.next().expect("three arguments");
                let c = it.next().expect("three arguments");
                for other in [&b, &c] {
                    if !fits(a.dim, other.dim) {
                        return Err(ExprError::at(
                            other.col,
                            format!(
                                "{n} takes three of a kind, not {} and {}",
                                name(a.dim),
                                name(other.dim)
                            ),
                        ));
                    }
                }
                let (f, dim) = if n == "clamp" {
                    (F3::Clamp, join(join(a.dim, b.dim), c.dim))
                } else {
                    (F3::Smoothstep, Some(Dim::NONE))
                };
                Ok(fold(Typed {
                    node: Node::Call3(f, Box::new(a), Box::new(b), Box::new(c)),
                    dim,
                    col,
                }))
            }
        }
    }
}

fn shown(t: &Token) -> String {
    if t.tok == Tok::End {
        "the end".to_owned()
    } else {
        format!("\"{}\"", t.text)
    }
}

fn branch(c: Typed, a: Typed, b: Typed, col: usize) -> Result<Typed, ExprError> {
    if !fits(a.dim, b.dim) {
        return Err(ExprError::at(
            b.col,
            format!(
                "the two values of an if must be of a kind, not {} and {}",
                name(a.dim),
                name(b.dim)
            ),
        ));
    }
    let dim = join(a.dim, b.dim);
    if let Node::Num(v) = c.node {
        return Ok(if v != 0.0 { a } else { b });
    }
    Ok(Typed {
        node: Node::If(Box::new(c), Box::new(a), Box::new(b)),
        dim,
        col,
    })
}

fn binary(op: Bin, a: Typed, b: Typed, dim: Option<Dim>, col: usize) -> Typed {
    fold(Typed {
        node: Node::Binary(op, Box::new(a), Box::new(b)),
        dim,
        col,
    })
}

/// A node whose arguments are all numbers, as the number it is.
fn fold(t: Typed) -> Typed {
    let num = |n: &Node| match n {
        Node::Num(v) => Some(*v),
        _ => None,
    };
    let v = match &t.node {
        Node::Unary(f, a) => num(&a.node).map(|x| f1(*f, x)),
        Node::Binary(op, a, b) => match (num(&a.node), num(&b.node)) {
            (Some(x), Some(y)) => Some(bin(*op, x, y)),
            _ => None,
        },
        Node::Call2(f, a, b) => match (num(&a.node), num(&b.node)) {
            (Some(x), Some(y)) => Some(f2(*f, x, y)),
            _ => None,
        },
        Node::Call3(f, a, b, c) => match (num(&a.node), num(&b.node), num(&c.node)) {
            (Some(x), Some(y), Some(z)) => Some(f3(*f, x, y, z)),
            _ => None,
        },
        _ => None,
    };
    match v {
        // a constant that isn't finite (1/0 is refused earlier, sqrt(-1) is here) is left to
        // the evaluation, which says where
        Some(v) if v.is_finite() => Typed {
            node: Node::Num(v),
            ..t
        },
        _ => t,
    }
}

fn bin(op: Bin, x: f64, y: f64) -> f64 {
    let b = |c: bool| if c { 1.0 } else { 0.0 };
    match op {
        Bin::Add => x + y,
        Bin::Sub => x - y,
        Bin::Mul => x * y,
        Bin::Div => x / y,
        Bin::Pow => pow(x, y),
        Bin::Lt => b(x < y),
        Bin::Le => b(x <= y),
        Bin::Gt => b(x > y),
        Bin::Ge => b(x >= y),
        Bin::Eq => b(x == y),
        Bin::Ne => b(x != y),
    }
}

/// x^y, by repeated multiplication for small whole exponents (exact for squares and cubes).
fn pow(x: f64, y: f64) -> f64 {
    if y.fract() == 0.0 && y.abs() <= 64.0 {
        x.powi(y as i32)
    } else {
        x.powf(y)
    }
}

fn f1(f: F1, x: f64) -> f64 {
    match f {
        F1::Neg => -x,
        F1::Sin => x.sin(),
        F1::Cos => x.cos(),
        F1::Tan => x.tan(),
        F1::Asin => x.asin(),
        F1::Acos => x.acos(),
        F1::Atan => x.atan(),
        F1::Exp => x.exp(),
        F1::Ln => x.ln(),
        F1::Log10 => x.log10(),
        F1::Sqrt => x.sqrt(),
        F1::Abs => x.abs(),
        F1::Floor => x.floor(),
        F1::Ceil => x.ceil(),
        F1::Round => x.round(),
        F1::Erf => erf(x),
        F1::Tanh => x.tanh(),
        F1::Sinc => {
            if x == 0.0 {
                1.0
            } else {
                x.sin() / x
            }
        }
    }
}

fn f2(f: F2, x: f64, y: f64) -> f64 {
    match f {
        F2::Atan2 => x.atan2(y),
        F2::Min => x.min(y),
        F2::Max => x.max(y),
    }
}

fn f3(f: F3, x: f64, lo: f64, hi: f64) -> f64 {
    match f {
        F3::Clamp => x.max(lo).min(hi),
        F3::Smoothstep => {
            // smoothstep(e0, e1, x): here (x, lo, hi) are (e0, e1, x)
            let u = ((hi - x) / (lo - x)).clamp(0.0, 1.0);
            u * u * (3.0 - 2.0 * u)
        }
    }
}

/// An instruction of the stack machine.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Const(f64),
    Load(u32),
    F1(F1),
    Bin(Bin),
    F2(F2),
    F3(F3),
    /// Pops the condition; jumps to the instruction if it is 0.
    JumpIfZero(u32),
    Jump(u32),
    /// Pops c, a, b; pushes a where c isn't 0, else b (both evaluated: for blocks).
    Select,
}

/// A compiled expression: parsed once, its units checked, its constants folded, and its
/// program a flat list of instructions. See [the module](super).
#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    text: String,
    dim: Option<Dim>,
    /// the program, with each instruction's column
    code: Vec<(Op, u32)>,
    /// the program for blocks of points: `if` as a select
    block: Vec<Op>,
    slots: Vec<usize>,
}

struct Emit {
    code: Vec<(Op, u32)>,
    block: Vec<Op>,
    depth: usize,
    most: usize,
    slots: Vec<usize>,
}

impl Emit {
    fn push(&mut self, op: Op, col: usize) {
        let delta: isize = match op {
            Op::Const(_) | Op::Load(_) => 1,
            Op::F1(_) | Op::Jump(_) => 0,
            Op::Bin(_) | Op::F2(_) | Op::JumpIfZero(_) => -1,
            Op::F3(_) | Op::Select => -2,
        };
        self.depth = self.depth.saturating_add_signed(delta);
        self.most = self.most.max(self.depth);
        self.code.push((op, col as u32));
    }

    fn node(&mut self, t: &Typed) {
        match &t.node {
            Node::Num(v) => {
                self.push(Op::Const(*v), t.col);
                self.block.push(Op::Const(*v));
            }
            Node::Load(s) => {
                if !self.slots.contains(s) {
                    self.slots.push(*s);
                }
                self.push(Op::Load(*s as u32), t.col);
                self.block.push(Op::Load(*s as u32));
            }
            Node::Unary(f, a) => {
                self.node(a);
                self.push(Op::F1(*f), t.col);
                self.block.push(Op::F1(*f));
            }
            Node::Binary(op, a, b) => {
                self.node(a);
                self.node(b);
                self.push(Op::Bin(*op), t.col);
                self.block.push(Op::Bin(*op));
            }
            Node::Call2(f, a, b) => {
                self.node(a);
                self.node(b);
                self.push(Op::F2(*f), t.col);
                self.block.push(Op::F2(*f));
            }
            Node::Call3(f, a, b, c) => {
                self.node(a);
                self.node(b);
                self.node(c);
                self.push(Op::F3(*f), t.col);
                self.block.push(Op::F3(*f));
            }
            Node::If(c, a, b) => {
                self.node(c);
                let jump = self.code.len();
                self.push(Op::JumpIfZero(0), t.col);
                self.node(a);
                let over = self.code.len();
                self.push(Op::Jump(0), t.col);
                // the else branch starts with the stack as it was before a
                self.depth -= 1;
                let start = self.code.len();
                self.node(b);
                let end = self.code.len();
                self.code[jump].0 = Op::JumpIfZero(start as u32);
                self.code[over].0 = Op::Jump(end as u32);
                self.block.push(Op::Select);
            }
        }
    }
}

impl Expr {
    /// Parses `text` with the names of `scope`, checks its units and compiles it.
    ///
    /// # Errors
    ///
    /// An [`ExprError`] at the column of what is wrong: a character or name it doesn't know, a
    /// misplaced operator or parenthesis, a function given the wrong number of arguments,
    /// units that don't add up, a division by a constant zero, or an expression that would need
    /// a stack deeper than 64 values.
    pub fn parse(text: &str, scope: &Scope) -> Result<Expr, ExprError> {
        let toks = lex(text)?;
        if toks.len() == 1 {
            return Err(ExprError::at(1, "is empty"));
        }
        let mut p = Parser { toks, k: 0, scope };
        let t = p.expr()?;
        if p.peek().tok != Tok::End {
            let t = p.peek();
            return Err(ExprError::at(
                t.col,
                format!("expected an operator or the end, found {}", shown(t)),
            ));
        }
        let mut e = Emit {
            code: Vec::new(),
            block: Vec::new(),
            depth: 0,
            most: 0,
            slots: Vec::new(),
        };
        e.node(&t);
        // the block program evaluates both sides of an if: its depth can be larger
        let block_depth = block_depth(&e.block);
        if e.most.max(block_depth) > DEPTH {
            return Err(ExprError::at(
                1,
                format!("is nested too deeply: it needs more than {DEPTH} values at once"),
            ));
        }
        e.slots.sort_unstable();
        Ok(Expr {
            text: text.to_owned(),
            dim: t.dim,
            code: e.code,
            block: e.block,
            slots: e.slots,
        })
    }

    /// The text it was parsed from.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Its dimension; `None` for a bare zero, which fits any.
    pub fn dim(&self) -> Option<Dim> {
        self.dim
    }

    /// The slots it reads.
    pub fn slots(&self) -> &[usize] {
        &self.slots
    }

    /// Its value if it reads no slot.
    pub fn constant(&self) -> Option<f64> {
        match self.code.as_slice() {
            [(Op::Const(v), _)] => Some(*v),
            _ => None,
        }
    }

    /// Its value at `values` (one per slot of the scope it was parsed with; lengths in µm,
    /// angles in radians), without allocating. A division by zero or a value outside a
    /// function's domain gives an infinity or NaN, as in floating point.
    ///
    /// # Panics
    ///
    /// If `values` is shorter than a slot it reads.
    pub fn eval(&self, values: &[f64]) -> f64 {
        self.run(values, false).unwrap_or(f64::NAN)
    }

    /// Its value at `values`, or where it stops being finite.
    ///
    /// # Errors
    ///
    /// An [`ExprError`] at the column of the operation that gave an infinity or a NaN from
    /// finite values: a division by zero, the root or logarithm of a negative number, asin of 2.
    ///
    /// # Panics
    ///
    /// If `values` is shorter than a slot it reads.
    pub fn try_eval(&self, values: &[f64]) -> Result<f64, ExprError> {
        self.run(values, true)
    }

    fn run(&self, values: &[f64], check: bool) -> Result<f64, ExprError> {
        let mut st = [0.0f64; DEPTH];
        let mut sp = 0usize;
        let mut pc = 0usize;
        while pc < self.code.len() {
            let (op, col) = self.code[pc];
            pc += 1;
            match op {
                Op::Const(v) => {
                    st[sp] = v;
                    sp += 1;
                }
                Op::Load(s) => {
                    st[sp] = values[s as usize];
                    sp += 1;
                }
                Op::F1(f) => {
                    let x = st[sp - 1];
                    let y = f1(f, x);
                    if check && !y.is_finite() && x.is_finite() {
                        return Err(domain(col, f, x));
                    }
                    st[sp - 1] = y;
                }
                Op::Bin(b) => {
                    let (x, y) = (st[sp - 2], st[sp - 1]);
                    let z = bin(b, x, y);
                    if check && !z.is_finite() && x.is_finite() && y.is_finite() {
                        return Err(ExprError::at(
                            col as usize,
                            match b {
                                Bin::Div if y == 0.0 => "division by zero".to_owned(),
                                Bin::Pow => format!("{x} to the power {y} isn't a number"),
                                _ => "overflows".to_owned(),
                            },
                        ));
                    }
                    sp -= 1;
                    st[sp - 1] = z;
                }
                Op::F2(f) => {
                    sp -= 1;
                    st[sp - 1] = f2(f, st[sp - 1], st[sp]);
                }
                Op::F3(f) => {
                    let (x, lo, hi) = (st[sp - 3], st[sp - 2], st[sp - 1]);
                    let z = f3(f, x, lo, hi);
                    if check && !z.is_finite() && x.is_finite() && lo.is_finite() && hi.is_finite()
                    {
                        return Err(ExprError::at(
                            col as usize,
                            "smoothstep's two edges are the same",
                        ));
                    }
                    sp -= 2;
                    st[sp - 1] = z;
                }
                Op::JumpIfZero(to) => {
                    sp -= 1;
                    if st[sp] == 0.0 {
                        pc = to as usize;
                    }
                }
                Op::Jump(to) => pc = to as usize,
                Op::Select => unreachable!("only in blocks"),
            }
        }
        Ok(st[0])
    }

    /// Its value and derivative along one direction of the parameters, given every slot's
    /// value and derivative ([`super::Parameters::derivatives`] gives them), by forward mode.
    /// `floor`, `ceil`, `round` and comparisons have derivative 0; `abs`, `min`, `max` and
    /// `clamp` the derivative of the side they take.
    ///
    /// # Panics
    ///
    /// If `values` is shorter than a slot it reads.
    pub fn eval_dual(&self, values: &[Dual]) -> Dual {
        let mut st = [Dual::default(); DEPTH];
        let mut sp = 0usize;
        let mut pc = 0usize;
        while pc < self.code.len() {
            let (op, _) = self.code[pc];
            pc += 1;
            match op {
                Op::Const(v) => {
                    st[sp] = Dual::constant(v);
                    sp += 1;
                }
                Op::Load(s) => {
                    st[sp] = values[s as usize];
                    sp += 1;
                }
                Op::F1(f) => st[sp - 1] = dual1(f, st[sp - 1]),
                Op::Bin(b) => {
                    sp -= 1;
                    st[sp - 1] = dual_bin(b, st[sp - 1], st[sp]);
                }
                Op::F2(f) => {
                    sp -= 1;
                    st[sp - 1] = dual2(f, st[sp - 1], st[sp]);
                }
                Op::F3(f) => {
                    sp -= 2;
                    st[sp - 1] = dual3(f, st[sp - 1], st[sp], st[sp + 1]);
                }
                Op::JumpIfZero(to) => {
                    sp -= 1;
                    if st[sp].value == 0.0 {
                        pc = to as usize;
                    }
                }
                Op::Jump(to) => pc = to as usize,
                Op::Select => unreachable!("only in blocks"),
            }
        }
        st[0]
    }

    /// Its values at many points at once: `values` for every slot, except that slot
    /// `varying[k].0` takes, at point n, the value `varying[k].1[n]`; the results into `out`,
    /// one per point. Each instruction runs over a block of 32 points, so the program is read
    /// once a block; both sides of an `if` are evaluated, and the one asked for kept.
    ///
    /// # Panics
    ///
    /// If a varying slice is shorter than `out`, or `values` shorter than a slot it reads.
    pub fn eval_many(&self, values: &[f64], varying: &[(usize, &[f64])], out: &mut [f64]) {
        let mut st = [[0.0f64; BLOCK]; DEPTH];
        for (start, chunk) in out.chunks_mut(BLOCK).enumerate() {
            let start = start * BLOCK;
            let n = chunk.len();
            let mut sp = 0usize;
            for op in &self.block {
                match *op {
                    Op::Const(v) => {
                        st[sp][..n].fill(v);
                        sp += 1;
                    }
                    Op::Load(s) => {
                        let s = s as usize;
                        match varying.iter().find(|v| v.0 == s) {
                            Some((_, xs)) => st[sp][..n].copy_from_slice(&xs[start..start + n]),
                            None => st[sp][..n].fill(values[s]),
                        }
                        sp += 1;
                    }
                    Op::F1(f) => {
                        for v in &mut st[sp - 1][..n] {
                            *v = f1(f, *v);
                        }
                    }
                    Op::Bin(b) => {
                        sp -= 1;
                        let (lo, hi) = st.split_at_mut(sp);
                        let (x, y) = (&mut lo[sp - 1], &hi[0]);
                        for m in 0..n {
                            x[m] = bin(b, x[m], y[m]);
                        }
                    }
                    Op::F2(f) => {
                        sp -= 1;
                        let (lo, hi) = st.split_at_mut(sp);
                        let (x, y) = (&mut lo[sp - 1], &hi[0]);
                        for m in 0..n {
                            x[m] = f2(f, x[m], y[m]);
                        }
                    }
                    Op::F3(_) | Op::Select => {
                        sp -= 2;
                        let (lo, hi) = st.split_at_mut(sp);
                        let x = &mut lo[sp - 1];
                        let (a, b) = (&hi[0], &hi[1]);
                        for m in 0..n {
                            x[m] = match *op {
                                Op::F3(f) => f3(f, x[m], a[m], b[m]),
                                _ => {
                                    if x[m] != 0.0 {
                                        a[m]
                                    } else {
                                        b[m]
                                    }
                                }
                            };
                        }
                    }
                    Op::JumpIfZero(_) | Op::Jump(_) => unreachable!("no jumps in blocks"),
                }
            }
            chunk.copy_from_slice(&st[0][..n]);
        }
    }
}

/// The stack a block program needs.
fn block_depth(code: &[Op]) -> usize {
    let (mut d, mut most) = (0isize, 0isize);
    for op in code {
        d += match op {
            Op::Const(_) | Op::Load(_) => 1,
            Op::Bin(_) | Op::F2(_) => -1,
            Op::F3(_) | Op::Select => -2,
            _ => 0,
        };
        most = most.max(d);
    }
    most as usize
}

fn domain(col: u32, f: F1, x: f64) -> ExprError {
    let what = match f {
        F1::Sqrt => format!("sqrt of a negative number, {x}"),
        F1::Ln | F1::Log10 => format!("the logarithm of {x}, which isn't positive"),
        F1::Asin | F1::Acos => format!("{x} is outside [-1, 1]"),
        _ => format!("{x} is outside the function's domain"),
    };
    ExprError::at(col as usize, what)
}

fn d(value: f64, derivative: f64) -> Dual {
    Dual { value, derivative }
}

fn dual1(f: F1, a: Dual) -> Dual {
    let x = a.value;
    let chain = |v: f64, slope: f64| {
        d(
            v,
            if a.derivative == 0.0 {
                0.0
            } else {
                slope * a.derivative
            },
        )
    };
    match f {
        F1::Neg => d(-x, -a.derivative),
        F1::Sin => chain(x.sin(), x.cos()),
        F1::Cos => chain(x.cos(), -x.sin()),
        F1::Tan => {
            let t = x.tan();
            chain(t, 1.0 + t * t)
        }
        F1::Asin => chain(x.asin(), 1.0 / (1.0 - x * x).sqrt()),
        F1::Acos => chain(x.acos(), -1.0 / (1.0 - x * x).sqrt()),
        F1::Atan => chain(x.atan(), 1.0 / (1.0 + x * x)),
        F1::Exp => {
            let e = x.exp();
            chain(e, e)
        }
        F1::Ln => chain(x.ln(), 1.0 / x),
        F1::Log10 => chain(x.log10(), 1.0 / (x * std::f64::consts::LN_10)),
        F1::Sqrt => {
            let r = x.sqrt();
            chain(r, 0.5 / r)
        }
        F1::Abs => chain(x.abs(), if x < 0.0 { -1.0 } else { 1.0 }),
        F1::Floor | F1::Ceil | F1::Round => d(f1(f, x), 0.0),
        F1::Erf => chain(erf(x), erf_derivative(x)),
        F1::Tanh => {
            let t = x.tanh();
            chain(t, 1.0 - t * t)
        }
        F1::Sinc => {
            if x.abs() < 1e-4 {
                // the series: 1 − x²/6 + x⁴/120, its derivative −x/3 + x³/30
                chain(
                    1.0 - x * x / 6.0 + x.powi(4) / 120.0,
                    -x / 3.0 + x.powi(3) / 30.0,
                )
            } else {
                chain(x.sin() / x, (x * x.cos() - x.sin()) / (x * x))
            }
        }
    }
}

fn dual_bin(op: Bin, a: Dual, b: Dual) -> Dual {
    let (x, y) = (a.value, b.value);
    match op {
        Bin::Add => d(x + y, a.derivative + b.derivative),
        Bin::Sub => d(x - y, a.derivative - b.derivative),
        Bin::Mul => d(x * y, a.derivative * y + x * b.derivative),
        Bin::Div => d(x / y, (a.derivative * y - x * b.derivative) / (y * y)),
        Bin::Pow => {
            let v = pow(x, y);
            let da = if a.derivative == 0.0 {
                0.0
            } else {
                y * pow(x, y - 1.0) * a.derivative
            };
            let db = if b.derivative == 0.0 {
                0.0
            } else {
                v * x.ln() * b.derivative
            };
            d(v, da + db)
        }
        _ => d(bin(op, x, y), 0.0),
    }
}

fn dual2(f: F2, a: Dual, b: Dual) -> Dual {
    match f {
        F2::Atan2 => {
            let (y, x) = (a.value, b.value);
            let r2 = x * x + y * y;
            d(y.atan2(x), (x * a.derivative - y * b.derivative) / r2)
        }
        F2::Min => {
            if a.value <= b.value {
                a
            } else {
                b
            }
        }
        F2::Max => {
            if a.value >= b.value {
                a
            } else {
                b
            }
        }
    }
}

fn dual3(f: F3, a: Dual, b: Dual, c: Dual) -> Dual {
    match f {
        F3::Clamp => {
            if a.value < b.value {
                b
            } else if a.value > c.value {
                c
            } else {
                a
            }
        }
        F3::Smoothstep => {
            // smoothstep(e0, e1, x) = s(u), u = (x − e0)/(e1 − e0)
            let (e0, e1, x) = (a, b, c);
            let w = e1.value - e0.value;
            let u = (x.value - e0.value) / w;
            if !(0.0..=1.0).contains(&u) {
                return d(u.clamp(0.0, 1.0).round(), 0.0);
            }
            let du = ((x.derivative - e0.derivative) * w
                - (x.value - e0.value) * (e1.derivative - e0.derivative))
                / (w * w);
            d(u * u * (3.0 - 2.0 * u), 6.0 * u * (1.0 - u) * du)
        }
    }
}

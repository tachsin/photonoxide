//! The verdict types of the validation harness: a [`Case`], what it measured ([`Outcome`]), what
//! it is checked against ([`Tier`]) and its [`Rule`] (#544).
//!
//! They import nothing above themselves, so the layers below the validation cases (which run
//! the solvers) can state their cases' outcomes and rules. [`crate::validation`] re-exports
//! them, lists the cases and writes the report.

/// What a case is checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// A closed-form solution.
    Analytic,
    /// An established code on the same structure.
    CrossCode,
    /// A published measurement or result.
    Published,
}

impl Tier {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Tier::Analytic => "analytic",
            Tier::CrossCode => "cross-code",
            Tier::Published => "published",
        }
    }
}

/// What a case measured.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    /// The value photonoxide computed, as reported.
    pub measured: f64,
    /// The value it should be.
    pub expected: f64,
    /// The largest error allowed: absolute, in the value's own unit.
    pub tolerance: f64,
    /// The actual error, |measured − expected| or a norm over several values.
    pub error: f64,
}

impl Outcome {
    /// Whether the error is within the tolerance.
    pub fn passed(&self) -> bool {
        self.error.is_finite() && self.error <= self.tolerance
    }
}

/// How a case's error is measured (#544).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Measure {
    /// \|measured − expected\| / \|expected\|; for a case whose expected value is 0, the
    /// relative error it reports.
    Relative,
    /// \|measured − expected\| in the value's own unit: where the expected value is zero by
    /// nature (a residual, an imbalance, an order's distance from the scheme's) or the
    /// reference states an absolute uncertainty.
    Absolute,
}

/// Where a case's tolerance comes from (#544).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Basis {
    /// An exact discrete identity: the arithmetic's round-off, at most 1e-8 relative in f64
    /// (about the square root of its unit round-off), 2e-4 in f32 ([`Rule::in_f32`]).
    RoundOff,
    /// A closed form against our grid: the grid's own error, its order checked by another case.
    Discretisation,
    /// A published number: the paper's stated uncertainty, or half a unit of its last printed
    /// digit where it states none.
    Reference,
    /// A published approximate model: the accuracy its paper gives it.
    Model,
    /// A convergence test: the scheme's order, within a margin.
    Order,
}

impl Basis {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Basis::RoundOff => "round-off",
            Basis::Discretisation => "discretisation",
            Basis::Reference => "reference",
            Basis::Model => "model",
            Basis::Order => "order",
        }
    }
}

/// Where a case's own numerical error comes from, shown beside its tolerance (#544).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Numerical {
    /// No grid and no iteration to converge: the arithmetic's round-off only.
    Exact,
    /// What the case measures is our grid's own error, or its order.
    Measured,
    /// Noted by the case as it runs, in the rule's measure: its grid study's estimate.
    Noted,
    /// Measured by a test elsewhere, in the rule's measure: its value, and where.
    Stated(f64, &'static str),
}

/// A case's rule (#544): how its error is measured, where its tolerance comes from and what it
/// is in that measure, and where our own numerical error comes from. Each area keeps its
/// cases' rules beside them, by the case's id ([`crate::validation::rules`]).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct Rule {
    /// How the case's error is measured.
    pub measure: Measure,
    /// Where its tolerance comes from.
    pub basis: Basis,
    /// The tolerance, in `measure`: the case's own, converted (relative to its expected value
    /// for [`Measure::Relative`]).
    pub tolerance: f64,
    /// Where our own numerical error comes from.
    pub numerical: Numerical,
    /// The tolerance's origin, in words: a paper's statement, a printed digit, a grid study.
    pub why: &'static str,
    /// The case's error is set by a platform's last bits (a fit, an iteration count, a
    /// noise-level difference): the report shows it as a bound below a third of the tolerance,
    /// so it reads the same on every system ([`Rule::varying`]).
    pub varies: bool,
    /// The case computes in f32, so a round-off tolerance may be up to 2e-4 ([`Rule::in_f32`]).
    pub single: bool,
}

impl Rule {
    /// A rule: its measure, basis, tolerance in that measure, our own error's origin, and why.
    pub const fn new(
        measure: Measure,
        basis: Basis,
        tolerance: f64,
        numerical: Numerical,
        why: &'static str,
    ) -> Rule {
        Rule {
            measure,
            basis,
            tolerance,
            numerical,
            why,
            varies: false,
            single: false,
        }
    }

    /// The same rule for a case whose error a platform's last bits set: an iterative fit's
    /// residual, say, which moves by half itself between systems. Its Error cell reads "under a
    /// third of it" while the error is below a third of the tolerance, the value above.
    pub const fn varying(self) -> Rule {
        Rule {
            varies: true,
            ..self
        }
    }

    /// The same rule for a case that computes in f32 (the GPU's): its round-off limit is f32's,
    /// 2e-4 relative, as 1e-8 is f64's.
    pub const fn in_f32(self) -> Rule {
        Rule {
            single: true,
            ..self
        }
    }

    /// What's wrong with the case's outcome against this rule, if anything: a tolerance other
    /// than the rule's, a round-off tolerance above 1e-8 (2e-4 in f32), a published comparison without our
    /// own error beside it, or our own error above a third of the tolerance.
    pub(crate) fn fault(&self, o: &Outcome, noted: Option<f64>) -> Option<String> {
        let own = if o.expected == 0.0 || self.measure == Measure::Absolute {
            o.tolerance
        } else {
            o.tolerance / o.expected.abs()
        };
        if (own - self.tolerance).abs() > 1e-9 * self.tolerance.abs().max(f64::MIN_POSITIVE) {
            return Some(format!(
                "its tolerance {own:e} isn't the rule's {:e}",
                self.tolerance
            ));
        }
        let limit = if self.single { 2e-4 } else { 1e-8 };
        if self.basis == Basis::RoundOff
            && self.measure == Measure::Relative
            && self.tolerance > limit
        {
            return Some(format!("a round-off tolerance above {limit:e}"));
        }
        if matches!(self.basis, Basis::Reference | Basis::Model)
            && self.numerical == Numerical::Measured
        {
            return Some("a published comparison needs our own error beside it".into());
        }
        let ours = match self.numerical {
            Numerical::Noted => match noted {
                Some(v) => Some(v),
                None => return Some("it noted no numerical error".into()),
            },
            Numerical::Stated(v, _) => Some(v),
            Numerical::Exact | Numerical::Measured => None,
        };
        // ours and the tolerance are in the same measure
        if let Some(ours) = ours
            && ours > self.tolerance / 3.0
        {
            return Some(format!(
                "our own error {ours:.1e} is above a third of the tolerance"
            ));
        }
        None
    }
}

thread_local! {
    static NOTED: std::cell::Cell<Option<f64>> = const { std::cell::Cell::new(None) };
}

/// Notes the case's own numerical error while it runs, in its rule's measure (relative, or in
/// the value's unit): its grid study's estimate. The report shows it beside the tolerance, and
/// fails a case whose error is above a third of it.
pub(crate) fn note_numerical(error: f64) {
    NOTED.with(|n| n.set(Some(error)));
}

/// Forgets what the last case noted, before the next one runs.
pub(crate) fn clear_noted() {
    NOTED.with(|n| n.set(None));
}

/// What the case that just ran noted, taken.
pub(crate) fn take_noted() -> Option<f64> {
    NOTED.with(|n| n.take())
}

/// One validation case.
///
/// Its `title` and `source` are cells of the report's Markdown table: their math is inline TeX
/// between single dollar signs, which GitHub, the studio and the project site render. A cell can't
/// hold a `|` (an absolute value is `\lvert x \rvert`), and GitHub drops the backslash before
/// punctuation, so `\,` and `\{` don't survive (`\thinspace`, `\lbrace` do), nor does a `>` (`\gt`).
#[derive(Clone, Copy, Debug)]
pub struct Case {
    /// A short, stable identifier, e.g. `"material/silicon-li-table"`.
    pub id: &'static str,
    /// What is checked, in a sentence.
    pub title: &'static str,
    /// What it is checked against.
    pub tier: Tier,
    /// The source of the expected value: a reference with its DOI, or the closed form.
    pub source: &'static str,
    /// Runs the case.
    pub run: fn() -> Outcome,
}

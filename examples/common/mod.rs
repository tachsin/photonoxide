//! What every example shares: comparing what photonoxide computes with what a paper prints.

// each example compiles this module on its own and uses only part of it
#![allow(dead_code)]

pub mod circuit;

use std::process::ExitCode;

/// The comparisons of one example. Each prints a row; [`Checks::finish`] fails the example
/// when any of them is outside its tolerance.
#[derive(Default)]
pub struct Checks {
    failed: usize,
    total: usize,
}

impl Checks {
    /// Compares `got` with the paper's `printed` value, allowing `tolerance` (absolute).
    pub fn compare(&mut self, what: &str, got: f64, printed: f64, tolerance: f64) {
        let ok = (got - printed).abs() <= tolerance;
        self.row(
            ok,
            format!("  {what:<34} {got:>12.6}   paper {printed:<10} ± {tolerance:e}"),
        );
    }

    /// Compares a count, e.g. of guided modes, with the paper's.
    pub fn count(&mut self, what: &str, got: usize, printed: usize) {
        self.row(
            got == printed,
            format!("  {what:<34} {got:>12}   paper {printed}"),
        );
    }

    fn row(&mut self, ok: bool, text: String) {
        self.total += 1;
        if !ok {
            self.failed += 1;
        }
        println!("{text:<80} {}", if ok { "ok" } else { "FAILED" });
    }

    /// Prints the summary; a failure if any comparison failed.
    pub fn finish(self) -> ExitCode {
        if self.failed == 0 {
            println!("all {} within tolerance of the paper", self.total);
            ExitCode::SUCCESS
        } else {
            println!("{} of {} outside tolerance", self.failed, self.total);
            ExitCode::FAILURE
        }
    }
}

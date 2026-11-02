//! The references index without building the studio: the studio's `references` module, which
//! uses the standard library alone, compiled by itself.
//!
//! - `just references` (or `rustc --edition 2024 -O --out-dir target/references
//!   .github/scripts/references.rs`, then `target/references/references --write
//!   docs/references.md`) writes docs/references.md, as `photonoxide references` does;
//! - CI's Docs job compiles it with `--test` and runs the module's tests, among them the check of
//!   the repository: the page is current, every cited source has an entry in
//!   docs/references.yaml and every entry is cited, every DOI is well formed.

#[path = "../../studio/src-tauri/src/references.rs"]
mod references;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    references::run(&args)
}

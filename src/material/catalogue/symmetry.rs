//! Which elements of d_il and r_ij a point group allows, and which are equal.
//!
//! The patterns are the standard ones (J. F. Nye, *Physical Properties of Crystals*, Oxford
//! (1957); R. W. Boyd, *Nonlinear Optics*), in the axes the catalogue's sources use: for 3m the
//! mirror plane is perpendicular to x₁ (so d₂₂ and r₂₂ are allowed), for 6mm and 3m the
//! threefold or sixfold axis is x₃. No Kleinman symmetry is assumed: d₁₅ and d₃₁ are separate
//! elements, equal only when Kleinman's condition holds, which a source must say.

use serde::Serialize;

use super::{Cell, TensorKind};

/// An element of a point group's pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Pattern {
    /// Zero by symmetry.
    Zero,
    /// An independent element.
    Independent,
    /// `sign` times another element (1-based row and column).
    Same {
        /// The other element's row.
        row: u8,
        /// The other element's column.
        col: u8,
        /// +1 or −1.
        sign: i8,
    },
}

const Z: Pattern = Pattern::Zero;
const I: Pattern = Pattern::Independent;
const fn s(row: u8, col: u8, sign: i8) -> Pattern {
    Pattern::Same { row, col, sign }
}

/// The pattern of d (3 × 6) or r (6 × 3) for a point group, or `None` for a point group the
/// catalogue doesn't use. Centrosymmetric groups (`"m-3m"`, `"∞∞m"`) give all zeros.
pub fn pattern(kind: TensorKind, point_group: &str) -> Option<Vec<Vec<Pattern>>> {
    let d: Vec<[Pattern; 6]> = match point_group {
        "m-3m" | "∞∞m" => vec![[Z; 6]; 3],
        // d14 = d25 = d36
        "-43m" => vec![
            [Z, Z, Z, I, Z, Z],
            [Z, Z, Z, Z, s(1, 4, 1), Z],
            [Z, Z, Z, Z, Z, s(1, 4, 1)],
        ],
        // d15, d16 = -d22; d21 = -d22, d22, d24 = d15; d31, d32 = d31, d33
        "3m" => vec![
            [Z, Z, Z, Z, I, s(2, 2, -1)],
            [s(2, 2, -1), I, Z, s(1, 5, 1), Z, Z],
            [I, s(3, 1, 1), I, Z, Z, Z],
        ],
        // d15, d24 = d15; d31, d32 = d31, d33
        "6mm" => vec![
            [Z, Z, Z, Z, I, Z],
            [Z, Z, Z, s(1, 5, 1), Z, Z],
            [I, s(3, 1, 1), I, Z, Z, Z],
        ],
        _ => return None,
    };
    Some(match kind {
        TensorKind::SecondOrder => d.into_iter().map(|row| row.to_vec()).collect(),
        TensorKind::ElectroOptic => {
            let r: Vec<[Pattern; 3]> = match point_group {
                "m-3m" | "∞∞m" => vec![[Z; 3]; 6],
                // r41 = r52 = r63
                "-43m" => vec![
                    [Z, Z, Z],
                    [Z, Z, Z],
                    [Z, Z, Z],
                    [I, Z, Z],
                    [Z, s(4, 1, 1), Z],
                    [Z, Z, s(4, 1, 1)],
                ],
                // r12 = -r22, r13; r22, r23 = r13; r33; r42 = r51; r51; r61 = -r22
                "3m" => vec![
                    [Z, s(2, 2, -1), I],
                    [Z, I, s(1, 3, 1)],
                    [Z, Z, I],
                    [Z, s(5, 1, 1), Z],
                    [I, Z, Z],
                    [s(2, 2, -1), Z, Z],
                ],
                // r13, r23 = r13, r33, r42 = r51, r51
                "6mm" => vec![
                    [Z, Z, I],
                    [Z, Z, s(1, 3, 1)],
                    [Z, Z, I],
                    [Z, s(5, 1, 1), Z],
                    [I, Z, Z],
                    [Z, Z, Z],
                ],
                _ => return None,
            };
            r.into_iter().map(|row| row.to_vec()).collect()
        }
    })
}

/// The cells of a tensor: the pattern with the given independent values (1-based row,
/// column, value in pm/V, uncertainty); independent elements not given are `Unknown`.
///
/// # Panics
///
/// If the point group is unknown, or a value is given for an element that isn't independent:
/// both are mistakes in the built-in data, which the tests catch.
pub(crate) fn cells(
    kind: TensorKind,
    point_group: &str,
    values: &[(u8, u8, f64, Option<f64>)],
) -> Vec<Vec<Cell>> {
    let p = pattern(kind, point_group).expect("a point group the catalogue knows");
    for &(r, c, ..) in values {
        assert_eq!(
            p[r as usize - 1][c as usize - 1],
            Pattern::Independent,
            "{point_group}: element {r}{c} isn't independent"
        );
    }
    p.iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, e)| match *e {
                    Pattern::Zero => Cell::Zero,
                    Pattern::Same { row, col, sign } => Cell::Same { row, col, sign },
                    Pattern::Independent => values
                        .iter()
                        .find(|v| v.0 as usize == i + 1 && v.1 as usize == j + 1)
                        .map_or(Cell::Unknown, |v| Cell::Value {
                            value: v.2,
                            uncertainty: v.3,
                        }),
                })
                .collect()
        })
        .collect()
}

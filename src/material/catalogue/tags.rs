//! The tags an entry shows above its data: what it is, its crystal's symmetry and what that
//! symmetry allows, each with a statement of what it means and where that comes from.
//!
//! The symmetry tags are written from the point group, and what a non-centrosymmetric group
//! allows is read from [`pattern`], the same table the tensors follow, so the tags and the
//! tensors can't disagree.

use serde::Serialize;

use super::{
    Category, Crystal, CrystalSystem, Entry, OpticalClass, Pattern, TensorKind, pattern,
};

/// One tag of an entry: a short statement and what it means.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Tag {
    /// What the tag is about: `"category"`, `"system"`, `"point-group"`, `"space-group"`,
    /// `"symmetry"` or `"optical"`.
    pub kind: String,
    /// The tag, in text with LaTeX between `$`s, e.g. `"$\bar{4}3m$ ($T_d$)"`.
    pub label: String,
    /// What it means, in text with LaTeX between `$`s.
    pub detail: String,
    /// Where the statement comes from, with DOIs; empty when it only describes the entry.
    pub source: String,
}

fn tag(kind: &str, label: impl Into<String>, detail: impl Into<String>, source: &str) -> Tag {
    Tag {
        kind: kind.into(),
        label: label.into(),
        detail: detail.into(),
        source: source.into(),
    }
}

const NYE: &str = "J. F. Nye, Physical Properties of Crystals (Oxford, 1957): the 32 classes in both notations and the forms of their tensors";
const ROBERTS: &str = "D. A. Roberts, IEEE J. Quantum Electron. 28, 2057 (1992), doi:10.1109/3.159516";
const ITA: &str = "International Tables for Crystallography, Vol. A (2016), doi:10.1107/97809553602060000114";
const BOYD: &str = "R. W. Boyd, Nonlinear Optics, 3rd ed. (Academic, 2008), Ch. 1, doi:10.1016/B978-0-12-369470-6.00001-0";
const SINATKAS: &str = "G. Sinatkas, T. Christopoulos, O. Tsilipakos, E. E. Kriezis, J. Appl. Phys. 130, 010901 (2021), doi:10.1063/5.0048712, pp. 010901-3 and -12";
const TIMURDOGAN: &str = "E. Timurdogan, C. V. Poulton, M. J. Byrd, M. R. Watts, Nat. Photonics 11, 200 (2017), doi:10.1038/nphoton.2017.14";

/// The Hermann–Mauguin symbol in LaTeX, the Schoenflies symbol in LaTeX and a sentence on the
/// group, for a point group the catalogue uses.
fn point_group_symbols(hm: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match hm {
        "m-3m" => (
            r"m\bar{3}m",
            r"O_h",
            "the full symmetry of the cube: 48 operations, among them the inversion",
        ),
        "-43m" => (
            r"\bar{4}3m",
            r"T_d",
            "the symmetry of the regular tetrahedron: 24 operations, without the inversion",
        ),
        "3m" => (
            r"3m",
            r"C_{3v}",
            "a threefold axis ($z$, the $c$ axis) and three mirror planes containing it: 6 operations, without the inversion",
        ),
        "6mm" => (
            r"6mm",
            r"C_{6v}",
            "a sixfold axis ($z$, the $c$ axis) and mirror planes containing it: 12 operations, without the inversion",
        ),
        "∞∞m" => (
            r"\infty\infty m",
            r"K_h",
            "the symmetry of a sphere: every rotation, and the inversion",
        ),
        _ => return None,
    })
}

/// A space group's printed symbol, e.g. `"Fd-3m (No. 227)"`, in LaTeX: `"Fd\bar{3}m"`, and
/// its number.
fn space_group_latex(space_group: &str) -> (String, Option<&str>) {
    let (symbol, number) = match space_group.split_once(" (") {
        Some((s, n)) => (s, Some(n.trim_end_matches(')'))),
        None => (space_group, None),
    };
    let mut tex = String::new();
    let mut chars = symbol.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '-' => {
                if let Some(d) = chars.next() {
                    tex.push_str(&format!(r"\bar{{{d}}}"));
                }
            }
            '₃' => tex.push_str("_3"),
            c => tex.push(c),
        }
    }
    (tex, number)
}

/// A tensor element's name in LaTeX, e.g. `d_{14}`.
fn element(kind: TensorKind, row: u8, col: u8) -> String {
    let letter = match kind {
        TensorKind::SecondOrder => 'd',
        TensorKind::ElectroOptic => 'r',
    };
    format!("{letter}_{{{row}{col}}}")
}

/// What a point group allows of d or r, from [`pattern`]: each independent element with the
/// elements equal to it, e.g. `$d_{14} = d_{25} = d_{36}$`, and how many there are.
fn allowed(kind: TensorKind, point_group: &str) -> Option<(usize, Vec<String>)> {
    let p = pattern(kind, point_group)?;
    let mut groups: Vec<(u8, u8, Vec<String>)> = Vec::new();
    for (i, row) in p.iter().enumerate() {
        for (j, e) in row.iter().enumerate() {
            if *e == Pattern::Independent {
                groups.push((i as u8 + 1, j as u8 + 1, Vec::new()));
            }
        }
    }
    for (i, row) in p.iter().enumerate() {
        for (j, e) in row.iter().enumerate() {
            if let Pattern::Same { row, col, sign } = *e {
                let g = groups.iter_mut().find(|g| g.0 == row && g.1 == col)?;
                let name = element(kind, i as u8 + 1, j as u8 + 1);
                g.2.push(if sign < 0 { format!("-{name}") } else { name });
            }
        }
    }
    let n = groups.len();
    let parts = groups
        .into_iter()
        .map(|(r, c, same)| {
            let mut s = element(kind, r, c);
            for o in same {
                s.push_str(" = ");
                s.push_str(&o);
            }
            format!("${s}$")
        })
        .collect();
    Some((n, parts))
}

fn word(n: usize) -> String {
    ["no", "one", "two", "three", "four", "five", "six"]
        .get(n)
        .map_or_else(|| n.to_string(), |w| (*w).to_owned())
}

fn count(n: usize, one: &str) -> String {
    format!("{} independent {one}{}", word(n), if n == 1 { "" } else { "s" })
}

const CAVEATS: &str = "This holds in the electric-dipole approximation; what remains: electric-quadrupole and magnetic-dipole terms of the bulk (weak); surfaces and interfaces, which break the inversion (surface second-harmonic generation); inhomogeneous strain, which gives an effective $\\chi^{(2)}$ (strained-silicon waveguides; a few pm/V in simulations, Sinatkas et al.); a static field $E_0$, which gives an effective $\\chi^{(2)} \\propto \\chi^{(3)} E_0$ (electric-field-induced second-harmonic generation, used in silicon p–n junction devices, Timurdogan et al.). The third-order $\\chi^{(3)}$ (the Kerr effect) is allowed.";

fn symmetry(crystal: &Crystal) -> Tag {
    let pg = crystal.point_group.as_str();
    if crystal.system == CrystalSystem::Amorphous {
        return tag(
            "symmetry",
            r"isotropic on average ($\infty\infty m$)",
            format!("An amorphous solid has no long-range order: averaged over it, every rotation and the inversion are symmetries ($\\infty\\infty m$, a centrosymmetric group). Under the inversion each polar index changes sign, so a polar tensor of rank 3 obeys $T_{{ijk}} = -T_{{ijk}}$ and vanishes (Neumann's principle): the bulk $\\chi^{{(2)}}_{{ijk}}$ and Pockels $r_{{ijk}}$ are zero, as in a centrosymmetric crystal. {CAVEATS} Poling or a frozen-in field can also leave a glass with an effective $\\chi^{{(2)}}$."),
            &format!("{BOYD}; {SINATKAS}; {TIMURDOGAN}; {NYE}"),
        );
    }
    if crystal.centrosymmetric {
        return tag(
            "symmetry",
            "centrosymmetric (inversion centre)",
            format!("The point group contains the inversion. Under it each polar index changes sign, so a polar tensor of rank 3 obeys $T_{{ijk}} = -T_{{ijk}}$ and vanishes (Neumann's principle): no bulk $\\chi^{{(2)}}_{{ijk}}$ and no Pockels $r_{{ijk}}$. {CAVEATS}"),
            &format!("{BOYD}; {SINATKAS}; {TIMURDOGAN}; {NYE}"),
        );
    }
    let (Some((nd, d)), Some((nr, r))) = (
        allowed(TensorKind::SecondOrder, pg),
        allowed(TensorKind::ElectroOptic, pg),
    ) else {
        return tag("symmetry", "non-centrosymmetric", "The point group has no inversion: $\\chi^{(2)}$ and the Pockels effect are allowed.", NYE);
    };
    let axes = if pg == "3m" {
        " The axes: $z$ along the threefold axis, $x$ perpendicular to a mirror plane, as the catalogue's sources use them (Roberts 1992); Kleinman's symmetry, when a source shows it holds, adds $d_{15} = d_{31}$, which the catalogue doesn't assume."
    } else if pg == "6mm" {
        " The axes: $z$ along the sixfold axis. Kleinman's symmetry, when it holds, adds $d_{15} = d_{31}$, which the catalogue doesn't assume."
    } else {
        ""
    };
    tag(
        "symmetry",
        format!("non-centrosymmetric: {} $d$, {} $r$", word(nd), word(nr)),
        format!(
            "The point group has no inversion, so $\\chi^{{(2)}}$ and the Pockels effect are allowed. It leaves {}: {}; and {}: {} (Voigt notation, $d = \\chi^{{(2)}}/2$, $\\Delta(1/n^2)_i = \\sum_k r_{{ik}} E_k$); every other element is zero.{axes} The tensors below follow this pattern.",
            count(nd, "second-order coefficient"),
            d.join(", "),
            count(nr, "Pockels coefficient"),
            r.join(", "),
        ),
        &if pg == "3m" {
            format!("{NYE}; {ROBERTS}")
        } else {
            NYE.to_owned()
        },
    )
}

fn optical(crystal: &Crystal) -> Tag {
    match &crystal.optical {
        OpticalClass::Isotropic => tag(
            "optical",
            "optically isotropic",
            if crystal.system == CrystalSystem::Amorphous {
                "One refractive index for every direction and polarization: the dielectric tensor of an amorphous solid is, on average, a scalar."
            } else {
                "One refractive index for every direction and polarization: a cubic point group makes every second-rank tensor, the dielectric tensor among them, a scalar."
            },
            NYE,
        ),
        OpticalClass::Uniaxial {
            positive,
            optic_axis,
        } => tag(
            "optical",
            format!(
                "uniaxial, {}",
                if *positive { "positive" } else { "negative" }
            ),
            format!(
                "One optic axis, {optic_axis}: light polarized perpendicular to it sees the ordinary index $n_o$, light polarized along it the extraordinary index $n_e$; {} means $n_e {} n_o$.",
                if *positive { "positive" } else { "negative" },
                if *positive { ">" } else { "<" }
            ),
            NYE,
        ),
    }
}

impl Entry {
    /// The entry's tags, in the order the Materials page shows them: its category, crystal
    /// system and structure, point group (Hermann–Mauguin and Schoenflies), space group, what
    /// the symmetry allows of the second-order and Pockels tensors, and its optical class.
    pub fn tags(&self) -> Vec<Tag> {
        let c = &self.crystal;
        let mut tags = vec![match self.category {
            Category::Dielectric => tag(
                "category",
                "dielectric",
                "An insulator used for its index and low loss: a glass or a deposited film.",
                "",
            ),
            Category::Semiconductor => tag(
                "category",
                "semiconductor",
                "A crystal with a band gap of a few eV or less: transparent below the gap, absorbing above it. Each index model states the range it covers.",
                "",
            ),
            Category::NonlinearCrystal => tag(
                "category",
                "nonlinear crystal",
                "A non-centrosymmetric dielectric crystal used for its $\\chi^{(2)}$ and Pockels effect.",
                "",
            ),
        }];
        tags.push(match c.system {
            CrystalSystem::Amorphous => tag(
                "system",
                "amorphous",
                "No long-range order, so no crystal system or lattice.",
                "",
            ),
            system => {
                let (name, what) = match system {
                    CrystalSystem::Cubic => ("cubic", "three equal, orthogonal axes and four threefold axes along the cube's diagonals"),
                    CrystalSystem::Hexagonal => ("hexagonal", "one sixfold axis, the $c$ axis"),
                    CrystalSystem::Trigonal => ("trigonal", "one threefold axis, the $c$ axis"),
                    _ => ("crystalline", ""),
                };
                tag(
                    "system",
                    format!("{name}, {} structure", c.structure),
                    format!("The {name} crystal system: {what}. The structure: {}.", c.structure),
                    NYE,
                )
            }
        });
        if let Some((hm, schoenflies, what)) = point_group_symbols(&c.point_group) {
            tags.push(if c.system == CrystalSystem::Amorphous {
                tag(
                    "point-group",
                    format!("isotropic: ${hm}$ (${schoenflies}$)"),
                    format!("Not a crystallographic point group: the limiting group ${hm}$ (Schoenflies ${schoenflies}$), {what}, which an amorphous solid has on average."),
                    NYE,
                )
            } else {
                tag(
                    "point-group",
                    format!("${hm}$ (${schoenflies}$)"),
                    format!("Point group ${hm}$ in Hermann–Mauguin notation, ${schoenflies}$ in Schoenflies': {what}. The point group alone fixes which tensor elements vanish or are equal."),
                    NYE,
                )
            });
        }
        if let Some(sg) = &c.space_group {
            let (tex, number) = space_group_latex(sg);
            tags.push(tag(
                "space-group",
                match number {
                    Some(n) => format!("${tex}$ ({n})"),
                    None => format!("${tex}$"),
                },
                format!(
                    "Space group ${tex}${}: the point group with the lattice's translations, screw axes and glide planes.",
                    number.map_or(String::new(), |n| format!(", {n} in the International Tables"))
                ),
                ITA,
            ));
        }
        tags.push(symmetry(c));
        tags.push(optical(c));
        tags
    }
}

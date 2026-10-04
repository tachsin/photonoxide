//! What the catalogue doesn't have a number for yet, and where that number will come from.
//!
//! Each item names the property, the paper that measures it (with its DOI, checked on Crossref,
//! and whether it is open access, checked on OpenAlex), and what the papers in hand say about
//! it, with the places in them. An item without a paper is a property no published measurement
//! was found for. [`Entry::missing`] holds the same items in short form, written from this table,
//! so the two can't disagree.

use serde::Serialize;

use super::{Entry, Missing};

/// A property the catalogue will have once the paper that measures it is read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Coming {
    /// The property, e.g. `"r₄₁(x)"`.
    pub property: String,
    /// The paper it comes from, short, e.g. `"Glick et al. 1988"`; empty when no measurement
    /// was found.
    pub source: String,
    /// The paper's citation, or empty.
    pub citation: String,
    /// The paper's DOI, or empty.
    pub doi: String,
    /// Whether the paper is open access.
    pub open: bool,
    /// What the papers in hand have on it, with the places in them, and why this paper is the
    /// one; text with LaTeX between `$`s.
    pub detail: String,
}

struct Item {
    entry: &'static str,
    property: &'static str,
    source: &'static str,
    citation: &'static str,
    doi: &'static str,
    open: bool,
    detail: &'static str,
}

const ITEMS: &[Item] = &[
    Item {
        entry: "algaas",
        property: "d₁₄(x)",
        source: "Shoji, Kondo & Ito 2002",
        citation: "I. Shoji, T. Kondo, R. Ito, Opt. Quantum Electron. 34, 797 (2002)",
        doi: "10.1023/A:1016545417478",
        open: false,
        detail: r"Shipped: $d_{14} = 105 \pm 11$ pm/V at $x = 0.15$ (Ulsig et al. 2024, Table 1). In hand: Ohashi et al. (1993) give only $|d(x)/d(\mathrm{GaAs})|$ at 1.064 µm, plotted (Fig. 6, ±20%) with no table. Ulsig et al. (Table 1) quote $x = 0.20$ and $0.42$ from Ohashi's data put on the absolute scale of Shoji et al.; their 2002 review, from the same group, is where those numbers are.",
    },
    Item {
        entry: "algaas",
        property: "r₄₁(x)",
        source: "Glick, Reinhart & Martin 1988",
        citation: "M. Glick, F. K. Reinhart, D. Martin, J. Appl. Phys. 63, 5877 (1988)",
        doi: "10.1063/1.340279",
        open: false,
        detail: r"In hand: Adachi (1985) has no electro-optic section (Table IV has only the piezoelectric $e_{14}$ and $d_{14}$). Berseth et al. (1992, p. 2823) take $r_{41}$ linear in $x$ with AlAs's equal to GaP's, $-0.94$ pm/V: an assumption, used to correct their spacers, not a measurement. Glick et al. measure $r_{41}$ of an AlGaAs solid solution at 1.1523 µm and find it near the average of GaAs's and GaP's.",
    },
    Item {
        entry: "linbo3",
        property: "d₂₂",
        source: "Miller, Nordland & Bridenbaugh 1971",
        citation: "R. C. Miller, W. A. Nordland, P. M. Bridenbaugh, J. Appl. Phys. 42, 4145 (1971)",
        doi: "10.1063/1.1659746",
        open: false,
        detail: r"In hand: Shoji et al. (1997, Tables 6 and 10) and Choy & Byer (1976, Table VII) measure $d_{33}$ and $d_{31}$ only. Roberts (1992, Table VI) lists $d_{22} = 2.1$ pm/V at 1.064 µm, a compilation rescaled to his own standard; the measurement is Miller et al.'s, who determine $d_{31}$, $d_{22}$ and $d_{33}$ against the melt composition.",
    },
    Item {
        entry: "linbo3",
        property: "d₁₅",
        source: "",
        citation: "",
        doi: "",
        open: false,
        detail: r"In hand: no paper measures it. Roberts (1992, Table VI) sets $d_{15} = d_{31}$ by Kleinman's symmetry, which the catalogue doesn't assume; Miller et al. (1971) measure $d_{31}$, $d_{22}$ and $d_{33}$, not $d_{15}$. A direct measurement wasn't found (OpenAlex, October 2026).",
    },
    Item {
        entry: "linbo3-mgo",
        property: "r₁₃, r₃₃",
        source: "Akiyama, Nakano & Shoji 2017",
        citation: "K. Akiyama, S. Nakano, I. Shoji, CLEO Pacific Rim 2017, s1621",
        doi: "10.1109/CLEOPR.2017.8118808",
        open: false,
        detail: r"In hand: Jazbinšek & Zgonik (2002) fit nominally undoped congruent crystals only, and note that doping with Ce or Fe raises $r_{33}^T$ by more than 30% (p. 411), so the congruent tensor isn't carried over. Akiyama et al. measure $r_{33}$ and $r_{13}$ of MgO-doped and undoped congruent and stoichiometric crystals at 633 nm with one method, and find them nearly equal; Nakano et al. (Opt. Mater. Express 7, 939 (2017), open access) describe the method.",
    },
    Item {
        entry: "linbo3-mgo",
        property: "r₂₂",
        source: "Yonekura, Jin & Takizawa 2007",
        citation: "K. Yonekura, L. Jin, K. Takizawa, Opt. Rev. 14, 194 (2007)",
        doi: "10.1007/s10043-007-0194-y",
        open: false,
        detail: r"In hand: nothing on MgO-doped crystals. Yonekura et al. measure $r_{22}$ of undoped and 5% MgO-doped congruent lithium niobate against the wavelength.",
    },
    Item {
        entry: "ingap",
        property: "r₄₁",
        source: "",
        citation: "",
        doi: "",
        open: false,
        detail: r"In hand: Ueno et al. (1997) and Ahler et al. (2026, with its supplement) measure $d_{14}$ only. No measurement of InGaP's Pockels coefficient was found (OpenAlex and a web search, October 2026).",
    },
    Item {
        entry: "ingap",
        property: "n from 1.8 to 1.9 eV",
        source: "Kato, Adachi, Nakanishi & Ohtsuka 1994",
        citation: "H. Kato, S. Adachi, H. Nakanishi, K. Ohtsuka, Jpn. J. Appl. Phys. 33, 186 (1994)",
        doi: "10.1143/JJAP.33.186",
        open: false,
        detail: r"In hand: Ferrini et al.'s Sellmeier ends at 1.8 eV and their Table 3 starts at 1.9 eV. Schubert et al. (1995) cover 0.8–5.0 eV but only as plots (Figs. 2 and 4), with the gap at $E_G = 1.85 \pm 0.02$ eV (p. 3418). Kato et al. measure $(\mathrm{Al_xGa_{1-x}})_{0.5}\mathrm{In_{0.5}P}$ by ellipsometry from 1.2 to 5.5 eV and give a model for $n$ and $k$ at any $x$ and photon energy.",
    },
    Item {
        entry: "ingap",
        property: "bonded thin film",
        source: "",
        citation: "",
        doi: "",
        open: false,
        detail: r"In hand: Ahler et al. measure the film's index before and after bonding by ellipsometry (Supplement 1, Fig. S4(b)) but show it only as a plot; their Zenodo deposit (doi:10.5281/zenodo.17748661) holds cut-back loss data, with no licence file. Thiel et al. (Appl. Phys. Lett. 125, 131102 (2024), open access) use Tanaka et al.'s model. The bonded film's model comes when it is published.",
    },
    Item {
        entry: "inp",
        property: "d₁₄ (absolute, from SHG)",
        source: "Shoji, Kondo & Ito 2002",
        citation: "I. Shoji, T. Kondo, R. Ito, Opt. Quantum Electron. 34, 797 (2002)",
        doi: "10.1023/A:1016545417478",
        open: false,
        detail: r"Shipped: $d_{14}(\mathrm{InP})/d_{14}(\mathrm{GaAs}) = 0.78 \pm 0.08$ at 10.55 µm (Lee & Fan 1974, Table I) and the electronic part of the Pockels nonlinearity, $d_{41}^E = 83$ pm/V at 1.064 µm (Suzuki & Tada 1984, Table II), which equals the SHG coefficient only where dispersion is negligible. Shoji et al.'s review puts semiconductors on the absolute scale of their 1997 paper (which has no InP); whether it has InP is to be checked.",
    },
    Item {
        entry: "algan",
        property: "n(x)",
        source: "Brunner et al. 1997",
        citation: "D. Brunner et al., J. Appl. Phys. 82, 5090 (1997)",
        doi: "10.1063/1.366309",
        open: false,
        detail: r"In hand: Rigler et al. (2013, Table II) fit each of nine films separately and give no model in $x$; the nine films are the models here. Brunner et al. describe the index as a function of photon energy, Al content ($0 \le x \le 1$) and temperature; Sanford et al. (J. Appl. Phys. 94, 2980 (2003), doi:10.1063/1.1598276) fit Sellmeier equations to prism-coupling data on twelve films.",
    },
    Item {
        entry: "algan",
        property: "d₃₁, d₃₃",
        source: "Sanford et al. 2005",
        citation: "N. A. Sanford et al., J. Appl. Phys. 97, 053512 (2005)",
        doi: "10.1063/1.1852695",
        open: false,
        detail: r"In hand: Rigler et al. (2013, p. 1) quote only literature ranges for $d_{33}$ of AlN and GaN. Sanford et al. measure $\chi^{(2)}_{31}$ and $\chi^{(2)}_{33}$ of AlₓGa₁₋ₓN films at 1064 nm by Maker fringes, and find that they fall roughly linearly with $x$.",
    },
    Item {
        entry: "algan",
        property: "r_ij",
        source: "",
        citation: "",
        doi: "",
        open: false,
        detail: r"In hand: nothing. No measurement of AlGaN's Pockels tensor was found (OpenAlex, October 2026); AlN's is in its own entry.",
    },
];

/// The items of an entry.
pub(crate) fn of(entry: &str) -> Vec<Coming> {
    ITEMS
        .iter()
        .filter(|i| i.entry == entry)
        .map(|i| Coming {
            property: i.property.into(),
            source: i.source.into(),
            citation: i.citation.into(),
            doi: i.doi.into(),
            open: i.open,
            detail: i.detail.into(),
        })
        .collect()
}

/// The items of an entry in [`Missing`]'s short form: `"from <paper>, coming"`.
pub(crate) fn missing(entry: &str) -> Vec<Missing> {
    of(entry)
        .into_iter()
        .map(|c| Missing {
            property: c.property,
            reason: if c.source.is_empty() {
                "coming when a measurement is published".into()
            } else {
                format!("from {}, coming", c.source)
            },
        })
        .collect()
}

/// The entry ids the items belong to, for the tests.
#[cfg(test)]
pub(crate) fn entries() -> Vec<&'static str> {
    ITEMS.iter().map(|i| i.entry).collect()
}

impl Entry {
    /// What the entry doesn't have a number for yet, each with the paper it will come from and
    /// what the papers in hand say; the same items as [`Entry::missing`], in full.
    pub fn coming(&self) -> Vec<Coming> {
        of(&self.id)
    }
}

//! Every paper the catalogue cites, with its DOI (each checked against Crossref).

use super::Reference;

/// (key, citation, title, DOI, open-access copy).
const PAPERS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "li-1980",
        "H. H. Li, J. Phys. Chem. Ref. Data 9, 561 (1980)",
        "Refractive index of silicon and germanium and its wavelength and temperature derivatives",
        "10.1063/1.555624",
        "",
    ),
    (
        "malitson-1965",
        "I. H. Malitson, J. Opt. Soc. Am. 55, 1205 (1965)",
        "Interspecimen comparison of the refractive index of fused silica",
        "10.1364/JOSA.55.001205",
        "",
    ),
    (
        "leviton-frey-2006",
        "D. B. Leviton, B. J. Frey, Proc. SPIE 6273, 62732K (2006)",
        "Temperature-dependent absolute refractive index measurements of synthetic fused silica",
        "10.1117/12.672853",
        "https://arxiv.org/abs/0805.0091",
    ),
    (
        "luke-2015",
        "K. Luke, Y. Okawachi, M. R. E. Lamont, A. L. Gaeta, M. Lipson, Opt. Lett. 40, 4823 (2015)",
        "Broadband mid-infrared frequency comb generation in a Si3N4 microresonator",
        "10.1364/OL.40.004823",
        "",
    ),
    (
        "zelmon-1997",
        "D. E. Zelmon, D. L. Small, D. Jundt, J. Opt. Soc. Am. B 14, 3319 (1997)",
        "Infrared corrected Sellmeier coefficients for congruently grown lithium niobate and 5 mol.% magnesium oxide-doped lithium niobate",
        "10.1364/JOSAB.14.003319",
        "",
    ),
    (
        "jazbinsek-zgonik-2002",
        "M. Jazbinšek, M. Zgonik, Appl. Phys. B 74, 407 (2002)",
        "Material tensor parameters of LiNbO3 relevant for electro- and elasto-optics",
        "10.1007/s003400200818",
        "",
    ),
    (
        "chelladurai-2025",
        "D. Chelladurai et al., Nat. Mater. 24, 868 (2025)",
        "Barium titanate and lithium niobate permittivity and Pockels coefficients from megahertz to sub-terahertz frequencies",
        "10.1038/s41563-025-02158-1",
        "https://arxiv.org/abs/2407.03443",
    ),
    (
        "shoji-1997",
        "I. Shoji, T. Kondo, A. Kitamoto, M. Shirane, R. Ito, J. Opt. Soc. Am. B 14, 2268 (1997)",
        "Absolute scale of second-order nonlinear-optical coefficients",
        "10.1364/JOSAB.14.002268",
        "",
    ),
    (
        "roberts-1992",
        "D. A. Roberts, IEEE J. Quantum Electron. 28, 2057 (1992)",
        "Simplified characterization of uniaxial and biaxial nonlinear optical crystals: a plea for standardization of nomenclature and conventions",
        "10.1109/3.159516",
        "",
    ),
    (
        "choy-byer-1976",
        "M. M. Choy, R. L. Byer, Phys. Rev. B 14, 1693 (1976)",
        "Accurate second-order susceptibility measurements of visible and infrared nonlinear crystals",
        "10.1103/PhysRevB.14.1693",
        "",
    ),
    (
        "jundt-1997",
        "D. H. Jundt, Opt. Lett. 22, 1553 (1997)",
        "Temperature-dependent Sellmeier equation for the index of refraction, n_e, in congruent lithium niobate",
        "10.1364/OL.22.001553",
        "",
    ),
    (
        "gayer-2008",
        "O. Gayer, Z. Sacks, E. Galun, A. Arie, Appl. Phys. B 91, 343 (2008); erratum Appl. Phys. B 101, 481 (2010), doi:10.1007/s00340-010-4203-7",
        "Temperature and wavelength dependent refractive index equations for MgO-doped congruent and stoichiometric LiNbO3",
        "10.1007/s00340-008-2998-2",
        "https://link.springer.com/content/pdf/10.1007/s00340-010-4203-7.pdf",
    ),
    (
        "gehrsitz-2000",
        "S. Gehrsitz, F. K. Reinhart, C. Gourgon, N. Herres, A. Vonlanthen, H. Sigg, J. Appl. Phys. 87, 7825 (2000)",
        "The refractive index of AlxGa1-xAs below the band gap: accurate determination and empirical modeling",
        "10.1063/1.373462",
        "https://www.dora.lib4ri.ch/psi/islandora/object/psi:70680",
    ),
    (
        "papatryfonos-2021",
        "K. Papatryfonos et al., AIP Adv. 11, 025327 (2021)",
        "Refractive indices of MBE-grown AlxGa(1-x)As ternary alloys in the transparent wavelength region",
        "10.1063/5.0039631",
        "https://doi.org/10.1063/5.0039631",
    ),
    (
        "skauli-2003",
        "T. Skauli et al., J. Appl. Phys. 94, 6447 (2003)",
        "Improved dispersion relations for GaAs and applications to nonlinear optics",
        "10.1063/1.1621740",
        "",
    ),
    (
        "skauli-2002",
        "T. Skauli et al., Opt. Lett. 27, 628 (2002)",
        "Measurement of the nonlinear coefficient of orientation-patterned GaAs and demonstration of highly efficient second-harmonic generation",
        "10.1364/OL.27.000628",
        "",
    ),
    (
        "berseth-1992",
        "C.-A. Berseth, C. Wuethrich, F. K. Reinhart, J. Appl. Phys. 71, 2821 (1992)",
        "The electro-optic coefficients of GaAs: measurements at 1.32 and 1.52 um and study of their dispersion between 0.9 and 10 um",
        "10.1063/1.351011",
        "",
    ),
    (
        "sugie-tada-1976",
        "M. Sugie, K. Tada, Jpn. J. Appl. Phys. 15, 421 (1976)",
        "Measurements of the linear electrooptic coefficients and analysis of the nonlinear susceptibilities in cubic GaAs and hexagonal CdS",
        "10.1143/JJAP.15.421",
        "",
    ),
    (
        "adachi-1985",
        "S. Adachi, J. Appl. Phys. 58, R1 (1985)",
        "GaAs, AlAs, and AlxGa1-xAs: material parameters for use in research and device applications",
        "10.1063/1.336070",
        "",
    ),
    (
        "afromowitz-1974",
        "M. A. Afromowitz, Solid State Commun. 15, 59 (1974)",
        "Refractive index of Ga1-xAlxAs",
        "10.1016/0038-1098(74)90014-3",
        "",
    ),
    (
        "ohashi-1993",
        "M. Ohashi et al., J. Appl. Phys. 74, 596 (1993)",
        "Determination of quadratic nonlinear optical coefficient of AlxGa1-xAs system by the method of reflected second harmonics",
        "10.1063/1.355272",
        "",
    ),
    (
        "ulsig-2024",
        "E. Z. Ulsig, M. L. Madsen, E. J. Stanton et al., Opt. Express 32, 36986 (2024)",
        "Efficient and widely tunable mid-infrared sources using GaAs and AlGaAs integrated platforms for second-order frequency conversion",
        "10.1364/OE.523615",
        "https://doi.org/10.1364/opticaopen.25540030.v1",
    ),
    (
        "tanaka-1986",
        "H. Tanaka, Y. Kawamura, H. Asahi, J. Appl. Phys. 59, 985 (1986)",
        "Refractive indices of In0.49Ga0.51-xAlxP lattice matched to GaAs",
        "10.1063/1.336581",
        "",
    ),
    (
        "schubert-1995",
        "M. Schubert, V. Gottschalch, C. M. Herzinger, H. Yao, P. G. Snyder, J. A. Woollam, J. Appl. Phys. 77, 3416 (1995)",
        "Optical constants of GaxIn1-xP lattice matched to GaAs",
        "10.1063/1.358632",
        "",
    ),
    (
        "ueno-1997",
        "Y. Ueno, V. Ricci, G. I. Stegeman, J. Opt. Soc. Am. B 14, 1428 (1997)",
        "Second-order susceptibility of Ga0.5In0.5P crystals at 1.5 um and their feasibility for waveguide quasi-phase matching",
        "10.1364/JOSAB.14.001428",
        "",
    ),
    (
        "ahler-2026",
        "L. Ahler et al., Optica 13, 1447 (2026)",
        "Low-loss InGaP-on-insulator waveguides for high-efficiency entangled pair generation and nonlinear photonics",
        "10.1364/OPTICA.589921",
        "https://doi.org/10.1364/OPTICA.589921",
    ),
    (
        "pettit-turner-1965",
        "G. D. Pettit, W. J. Turner, J. Appl. Phys. 36, 2081 (1965)",
        "Refractive index of InP",
        "10.1063/1.1714410",
        "",
    ),
    (
        "suzuki-tada-1984",
        "N. Suzuki, K. Tada, Jpn. J. Appl. Phys. 23, 291 (1984)",
        "Electrooptic properties and Raman scattering in InP",
        "10.1143/JJAP.23.291",
        "",
    ),
    (
        "lee-fan-1974",
        "C. C. Lee, H. Y. Fan, Phys. Rev. B 10, 703 (1974)",
        "Second-harmonic generation in InSb, InP, and AlSb",
        "10.1103/PhysRevB.10.703",
        "",
    ),
    (
        "pastrnak-roskovcova-1966",
        "J. Pastrňák, L. Roskovcová, Phys. Status Solidi 14, K5 (1966)",
        "Refraction index measurements on AlN single crystals",
        "10.1002/pssb.19660140127",
        "",
    ),
    (
        "majkic-2017",
        "A. Majkić et al., Phys. Status Solidi B 254, 1700077 (2017)",
        "Optical nonlinear and electro-optical coefficients in bulk aluminium nitride single crystals",
        "10.1002/pssb.201700077",
        "",
    ),
    (
        "graupner-1992",
        "P. Gräupner, J. C. Pommier, A. Cachard, J. L. Coutaz, J. Appl. Phys. 71, 4136 (1992)",
        "Electro-optical effect in aluminum nitride waveguides",
        "10.1063/1.350844",
        "",
    ),
    (
        "rigler-2015",
        "M. Rigler et al., Appl. Phys. Express 8, 042603 (2015)",
        "Optical characterization of Al- and N-polar AlN waveguides for integrated optics",
        "10.7567/APEX.8.042603",
        "",
    ),
    (
        "rigler-2013",
        "M. Rigler, M. Zgonik, M. P. Hoffmann, R. Kirste, M. Bobea et al., Appl. Phys. Lett. 102, 221106 (2013)",
        "Refractive index of III-metal-polar and N-polar AlGaN waveguides grown by metal organic chemical vapor deposition",
        "10.1063/1.4800554",
        "",
    ),
    (
        "ferrini-2002",
        "R. Ferrini, G. Guizzetti, M. Patrini, A. Parisini, L. Tarricone, B. Valenti, Eur. Phys. J. B 27, 449 (2002)",
        "Optical functions of InGaP/GaAs epitaxial layers from 0.01 to 5.5 eV",
        "10.1140/epjb/e2002-00177-x",
        "",
    ),
];

/// The papers with these keys.
///
/// # Panics
///
/// If a key isn't in the list: a mistake in the built-in data, which the tests catch.
pub(crate) fn references(keys: &[&str]) -> Vec<Reference> {
    keys.iter()
        .map(|key| {
            let &(key, citation, title, doi, oa) = PAPERS
                .iter()
                .find(|p| p.0 == *key)
                .unwrap_or_else(|| panic!("no reference {key}"));
            Reference {
                key: key.into(),
                citation: citation.into(),
                title: title.into(),
                doi: doi.into(),
                open_access: (!oa.is_empty()).then(|| oa.into()),
            }
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn all_keys() -> Vec<&'static str> {
    PAPERS.iter().map(|p| p.0).collect()
}

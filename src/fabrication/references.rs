//! The papers and books the process library cites, each by the key its PDF is kept under in the
//! papers folder, with its DOI (checked on Crossref) or, for a book Crossref doesn't hold, its
//! ISBN as printed in the copy read.

use serde::{Deserialize, Serialize};

/// A paper or book the library cites.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    /// The key, the file name its PDF is kept under without `.pdf`, e.g. `"bruel-1995"`.
    pub key: String,
    /// The citation.
    pub citation: String,
    /// The title.
    pub title: String,
    /// The DOI, checked on Crossref; empty for a book Crossref doesn't hold.
    pub doi: String,
    /// The ISBN, for a book.
    pub isbn: String,
    /// An open-access copy, when there is one.
    pub open_access: String,
}

/// `(key, citation, title, DOI, ISBN, open copy)`.
type Row = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

const ROWS: &[Row] = &[
    (
        "bruel-1995",
        "M. Bruel, Electron. Lett. 31, 1201 (1995)",
        "Silicon on insulator material technology",
        "10.1049/el:19950805",
        "",
        "",
    ),
    (
        "selvaraja-2009",
        "S. K. Selvaraja, P. Jaenen, W. Bogaerts, D. Van Thourhout, P. Dumon, R. Baets, J. Lightwave Technol. 27, 4076 (2009)",
        "Fabrication of Photonic Wire and Crystal Circuits in Silicon-on-Insulator Using 193-nm Optical Lithography",
        "10.1109/JLT.2009.2022282",
        "",
        "",
    ),
    (
        "bojko-2011",
        "R. J. Bojko, J. Li, L. He, T. Baehr-Jones, M. Hochberg, Y. Aida, J. Vac. Sci. Technol. B 29, 06F309 (2011)",
        "Electron beam lithography writing strategies for low loss, high confinement silicon optical waveguides",
        "10.1116/1.3653266",
        "",
        "",
    ),
    (
        "chen-2015",
        "Y. Chen, Microelectron. Eng. 135, 57 (2015)",
        "Nanofabrication by electron beam lithography and its applications: A review",
        "10.1016/j.mee.2015.02.042",
        "",
        "",
    ),
    (
        "chou-1996",
        "S. Y. Chou, P. R. Krauss, P. J. Renstrom, Science 272, 85 (1996)",
        "Imprint Lithography with 25-Nanometer Resolution",
        "10.1126/science.272.5258.85",
        "",
        "",
    ),
    (
        "donnelly-kornblit-2013",
        "V. M. Donnelly, A. Kornblit, J. Vac. Sci. Technol. A 31, 050825 (2013)",
        "Plasma etching: Yesterday, today, and tomorrow",
        "10.1116/1.4819316",
        "",
        "",
    ),
    (
        "krishnan-2009",
        "M. Krishnan, J. W. Nalaskowski, L. M. Cook, Chem. Rev. 110, 178 (2009)",
        "Chemical Mechanical Planarization: Slurry Chemistry, Materials, and Mechanisms",
        "10.1021/cr900170z",
        "",
        "",
    ),
    (
        "deal-grove-1965",
        "B. E. Deal, A. S. Grove, J. Appl. Phys. 36, 3770 (1965)",
        "General Relationship for the Thermal Oxidation of Silicon",
        "10.1063/1.1713945",
        "",
        "",
    ),
    (
        "levy-1998",
        "M. Levy, R. M. Osgood, R. Liu, L. E. Cross, G. S. Cargill, A. Kumar, H. Bakhru, Appl. Phys. Lett. 73, 2293 (1998)",
        "Fabrication of single-crystal lithium niobate films by crystal ion slicing",
        "10.1063/1.121801",
        "",
        "",
    ),
    (
        "poberaj-2012",
        "G. Poberaj, H. Hu, W. Sohler, P. Günter, Laser Photonics Rev. 6, 488 (2012)",
        "Lithium niobate on insulator (LNOI) for micro-photonic devices",
        "10.1002/lpor.201100035",
        "",
        "",
    ),
    (
        "ulliac-2011",
        "G. Ulliac, B. Guichardaz, J.-Y. Rauch, S. Queste, S. Benchabane, N. Courjal, Microelectron. Eng. 88, 2417 (2011)",
        "Ultra-smooth LiNbO3 micro and nano structures for photonic applications",
        "10.1016/j.mee.2011.02.024",
        "",
        "https://hal.science/hal-00583832",
    ),
    (
        "zhang-2017-optica",
        "M. Zhang, C. Wang, R. Cheng, A. Shams-Ansari, M. Lončar, Optica 4, 1536 (2017)",
        "Monolithic ultra-high-Q lithium niobate microring resonator",
        "10.1364/OPTICA.4.001536",
        "",
        "https://doi.org/10.1364/OPTICA.4.001536",
    ),
    (
        "krasnokutska-2018",
        "I. Krasnokutska, J.-L. J. Tambasco, X. Li, A. Peruzzo, Opt. Express 26, 897 (2018)",
        "Ultra-low loss photonic circuits in lithium niobate on insulator",
        "10.1364/OE.26.000897",
        "",
        "https://doi.org/10.1364/OE.26.000897",
    ),
    (
        "kaufmann-2023",
        "F. Kaufmann, G. Finco, A. Maeder, R. Grange, Nanophotonics 12, 1601 (2023)",
        "Redeposition-free inductively-coupled plasma etching of lithium niobate for integrated photonics",
        "10.1515/nanoph-2022-0676",
        "",
        "https://doi.org/10.1515/nanoph-2022-0676",
    ),
    (
        "zhang-2022",
        "Y. Zhang, L. Shao, J. Yang, Z. Chen, K. Zhang, K.-M. Shum, D. Zhu, C. H. Chan, M. Lončar, C. Wang, Photon. Res. 10, 2380 (2022)",
        "Systematic investigation of millimeter-wave optic modulation performance in thin-film lithium niobate",
        "10.1364/PRJ.468518",
        "",
        "https://doi.org/10.1364/PRJ.468518",
    ),
    (
        "jacques-2019",
        "M. Jacques, A. Samani, E. El-Fiky, D. Patel, Z. Xing, D. V. Plant, Opt. Express 27, 10456 (2019)",
        "Optimization of thermo-optic phase-shifter design and mitigation of thermal crosstalk on the SOI platform",
        "10.1364/OE.27.010456",
        "",
        "https://doi.org/10.1364/OE.27.010456",
    ),
    (
        "harris-2014",
        "N. C. Harris, Y. Ma, J. Mower, T. Baehr-Jones, D. Englund, M. Hochberg, C. Galland, Opt. Express 22, 10487 (2014)",
        "Efficient, compact and low loss thermo-optic phase shifter in silicon",
        "10.1364/OE.22.010487",
        "",
        "https://doi.org/10.1364/OE.22.010487",
    ),
    (
        "kearney-2018",
        "B. T. Kearney, B. Jugdersuren, J. C. Culbertson, P. A. DeSario, X. Liu, Thin Solid Films 661, 78 (2018)",
        "Substrate and annealing temperature dependent electrical resistivity of sputtered titanium nitride thin films",
        "10.1016/j.tsf.2018.07.001",
        "",
        "",
    ),
    (
        "rios-2015",
        "C. Ríos, M. Stegmaier, P. Hosseini, D. Wang, T. Scherer, C. D. Wright, H. Bhaskaran, W. H. P. Pernice, Nat. Photonics 9, 725 (2015)",
        "Integrated all-photonic non-volatile multi-level memory",
        "10.1038/nphoton.2015.182",
        "",
        "",
    ),
    (
        "delaney-2020",
        "M. Delaney, I. Zeimpekis, D. Lawson, D. W. Hewak, O. L. Muskens, Adv. Funct. Mater. 30, 2002447 (2020)",
        "A new family of ultralow loss reversible phase-change materials for photonic integrated circuits: Sb2S3 and Sb2Se3",
        "10.1002/adfm.202002447",
        "",
        "https://onlinelibrary.wiley.com/doi/pdfdirect/10.1002/adfm.202002447",
    ),
    (
        "madou-2018",
        "M. J. Madou, Fundamentals of Microfabrication and Nanotechnology, 3rd edition, Vol. II, Manufacturing Techniques for Microfabrication and Nanotechnology, CRC Press (2018; the copy read is the 2012 print, the same third edition)",
        "Fundamentals of Microfabrication and Nanotechnology",
        "10.1201/9781315274164",
        "978-1-4200-5519-1",
        "",
    ),
    (
        "campbell-2008",
        "S. A. Campbell, Fabrication Engineering at the Micro- and Nanoscale, 3rd edition, Oxford University Press (2008)",
        "Fabrication Engineering at the Micro- and Nanoscale",
        "",
        "978-0-19-532017-6",
        "",
    ),
];

/// Every reference the library cites, in the order above.
pub fn references() -> Vec<Reference> {
    ROWS.iter()
        .map(|&(key, citation, title, doi, isbn, open)| Reference {
            key: key.into(),
            citation: citation.into(),
            title: title.into(),
            doi: doi.into(),
            isbn: isbn.into(),
            open_access: open.into(),
        })
        .collect()
}

/// The reference with this key.
pub fn reference(key: &str) -> Option<Reference> {
    references().into_iter().find(|r| r.key == key)
}

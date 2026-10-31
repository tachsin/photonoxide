//! The numbers the rules themselves rest on, each with its source: the Rayleigh criterion's k₁,
//! the over-etch an etch to a stop needs, the aspect ratio at which narrow openings start to etch
//! slower, how closely a layer must register to the one below, and how a dimension's
//! contributions combine.
//!
//! - **Resolution** (Campbell, Eq. 7.18; Franssila, Eq. 10.1; Mack, Eq. 1.7): the smallest line
//!   a projection lithography prints is about W = k₁ λ / NA. k₁ "typically" runs from 0.8 down
//!   to 0.4, and its theoretical limit "is believed to be 0.25" (Campbell, p. 183), the smallest
//!   half-pitch with tilted illumination (Mack, Eq. 2.83).
//! - **Over-etch** (Franssila, pp. 138–139): an etch is run 10–20 % longer than the film's
//!   thickness needs, to clear it everywhere; what it then removes of the film below is that
//!   over-etch divided by the selectivity over it.
//! - **RIE lag** (Franssila, Section 21.7.1): openings of aspect ratio 1:1 etch
//!   straightforwardly; around 2:1 the narrower ones start to etch slower than the wider.
//! - **Registration** (Campbell, p. 194): a reasonable rule is that registration errors be no
//!   more than a quarter to a third of the resolution.
//! - **Combining contributions** (Campbell, Eq. 7.27): the total of independent components is the
//!   root of the sum of their squares; at worst they add (Franssila, p. 135, writes the oxide an
//!   etch removes below as the sum of four contributions).

use super::stated::{Source, Stated, Unit};

fn src(reference: &str, location: &str) -> Source {
    Source::new(reference, location)
}

/// k₁'s typical range, 0.4 to 0.8 (Campbell, Section 7.6, p. 183).
pub fn k1_typical() -> Stated {
    Stated::range(
        0.4,
        0.8,
        Unit::Number,
        src("campbell-2008", "Section 7.6, Eq. 7.18 (p. 183)"),
    )
}

/// k₁'s theoretical limit, 0.25 (Campbell, Section 7.6, p. 183; Mack's smallest half-pitch with
/// tilted illumination, Eq. 2.83, p. 58).
pub fn k1_limit() -> Stated {
    Stated::value(
        0.25,
        Unit::Number,
        src("campbell-2008", "Section 7.6, Eq. 7.18 (p. 183)"),
    )
}

/// Mack's statement of the same limit: the smallest half-pitch 0.25 λ/NA with tilted
/// illumination.
pub fn k1_limit_mack() -> Stated {
    Stated::value(
        0.25,
        Unit::Number,
        src("mack-2007", "Section 2.3.1, Eq. 2.83 (p. 58)"),
    )
}

/// The over-etch an etch to a stop is run with, 10–20 % of the film's etch time (Franssila,
/// Section 11.12.3, pp. 138–139).
pub fn over_etch() -> Stated {
    Stated::range(
        10.0,
        20.0,
        Unit::Percent,
        src("franssila-2010", "Section 11.12.3 (pp. 138–139)"),
    )
}

/// The aspect ratio around which narrow openings start to etch slower (RIE lag), 2:1; 1:1 is
/// straightforward (Franssila, Section 21.7.1, p. 266).
pub fn rie_lag() -> Stated {
    Stated::value(
        2.0,
        Unit::Ratio,
        src("franssila-2010", "Section 21.7.1 (p. 266)"),
    )
}

/// How large registration errors may be against the resolution: a quarter to a third (Campbell,
/// Section 7.9, p. 194).
pub fn registration() -> Stated {
    Stated::range(
        0.25,
        1.0 / 3.0,
        Unit::Number,
        src("campbell-2008", "Section 7.9 (p. 194)"),
    )
}

/// The etch-rate and film-thickness non-uniformity a textbook takes as its example, 5 % each,
/// when an etch's own is not stated (Franssila, Section 11.12.3, p. 138).
pub fn example_non_uniformity() -> Stated {
    Stated::value(
        5.0,
        Unit::Percent,
        src("franssila-2010", "Section 11.12.3 (p. 138)"),
    )
}

/// The rule that combines a dimension's contributions, in plain words, with its sources.
pub const COMBINATION: &str = "The contributions are combined as the root of the sum of their \
     squares, as the total of independent components is (campbell-2008, Section 7.9, Eq. 7.27, \
     p. 194); the worst case, every contribution at its bound at once, is their sum, as an \
     etch's loss of the film below is the sum of its contributions (franssila-2010, Section 11.9, \
     p. 135). Each contribution is its source's stated variation, read as a bound either way.";

/// Every source the rules cite, for the capability check.
pub fn sources() -> Vec<Source> {
    let mut out: Vec<Source> = [
        k1_typical(),
        k1_limit(),
        k1_limit_mack(),
        over_etch(),
        rie_lag(),
        registration(),
        example_non_uniformity(),
    ]
    .iter()
    .filter_map(|s| s.source().cloned())
    .collect();
    out.push(src("campbell-2008", "Section 7.9, Eq. 7.27 (p. 194)"));
    out.push(src("franssila-2010", "Section 11.9 (p. 135)"));
    out
}
